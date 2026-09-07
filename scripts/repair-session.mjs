#!/usr/bin/env node
/**
 * @file repair-session.mjs
 * @description DSH 会话自愈与修复工具（2026-09-04 重写：与上游加载器语义对齐；
 * 2026-09-07 二次修订：健康判定升级为 dsh 本尊恢复校验）。
 *
 * 背景：旧版按 turn 归流 + 全量重编号的修法会破坏 dsh 的 append-only 模型与
 * sourceEventSeqs 出处链，且对「重放重叠」这类真实损坏造成语义混乱；本版只做
 * 无损修复，绝不重编号、不重排、不新增/删除事件（除删除被遮蔽的旧占位事件）。
 *
 * 损坏类别（2026-09-04 实测两个真实会话，锚 dsh v0.1.2-rc.1 加载器
 * dsh-session-persistence-jsonl/lib/index.js consumeEventLine / zstd.js）：
 * 1. 重放重叠（replay overlap）：会话中断后恢复时，dsh 以相同 seq 重放被中断
 *    轮次的真实事件（旧「interrupted 占位」事件仍在磁盘上，形成
 *    「前缀 + 重放块」重叠）。加载器在重叠处报 `seq gap in committed region`
 *    并丢弃重叠点之后全部恢复事件。修复 = 丢弃被完全遮蔽的旧占位事件，
 *    保留重放块与其后全部事件（顺序与 seq 原样）。
 * 2. 序列缺失（missing seq）：中间事件缺失（异常中断/外部截断）。加载器只
 *    保留连续前缀。修复 = 按加载器语义截断到连续前缀（丢失部分本不可达）。
 * 3. 其他（JSON 不可解析 / 行语义损坏 / zstd 帧结构损坏）：不可安全修复，
 *    文件保持原样，报告失败。
 * 4. surface 语义损坏（2026-09-07 实测 session-8650d6f2）：存储层 seq 连续，
 *    但 surfaceOp.replace 的 start/end 指向当前 surface 不存在的节点（历史
 *    修复重编号未同步引用所致）。dsh 打开即抛
 *    `invalid seed event at index N: surface replace: end seq X not found in surface`
 *    ——旧健康检查只看 seq 连续性，对此类完全失明。修复 = 悬空 replace 转
 *    append + 剥离失效 sourceEventSeqs（消息内容与 seq 全保留）。
 *
 * 健康判定（2026-09-07 起的两层模型）：
 * - 第一层（存储层）：行展开后 seq 从 0 严格连续 + sourceEventSeqs 存储形
 *   合法（第 1/2/3 类检测，手写、锚加载器语义）；
 * - 第二层（恢复层）：用 dsh 引擎档自带的 @deepseek-ai/dsh-session（与当前
 *   安装的 dsh 同版本）执行真实恢复链——词汇表闸门 → adoptSessionEvent →
 *   interruptedTurnClosers 补尾 → Session.fromRestore（含 surface fold 全量
 *   重放）。这是 dsh 打开会话的确切路径，本层通过 = dsh 一定能加载。
 *   引擎档缺包时降级为内置 fold 移植（锚 surface.ts v0.1.2-rc.1），宁可
 *   降级也不回退到只看 seq 连续性——那正是本 bug 存活的缝隙。
 *
 * 格式版本路由（2026-09-07 二次修订）：
 * - dsh ≥0.1.3 起存储为「不可变世代」模型（v0 源 + session.vN 迁移产物并存，
 *   SESSION_FORMAT_VERSION=2），读取经 session-format-catalog 迁移管线。校验器
 *   检测到引擎档 catalog 时复刻同一路径：readHeader 分类 → decodeRecoverable
 *   Artifact → migrate → encodeCurrent → 迁移后 v2 表示走同一条扫描+恢复链；
 * - 存储版本比已装 dsh 新（或迁移器拒绝）→ 归类 unknown + 升级提示，**不是**
 *   needs_repair（不可修复、不尝试变异，与 dsh refuseForeignFormatVersion 同语义）；
 * - 无 catalog（0.1.2 代）时要求存储版本与已装 dsh 一致，其余版本同样归 unknown；
 * - fallback fold 仅锚 v0；v1+ 文件在 fallback 下同样归 unknown。
 * - 同一会话目录可能同时存在 v0 源与 vN 世代：本脚本按文件独立校验/修复
 *   （dsh 实际读取哪个世代由其 findLog 决定，壳端扫描展示时取最高世代）。
 *
 * 安全约束：
 * - 修复输出必须通过「存储层校验 + 恢复层校验」双闸门；任一失败 → 用内存中
 *   的原始字节回滚，退出码非 0；
 * - 无需修复（健康）→ 文件保持原样（不写回），退出码 0；
 * - 任何失败路径退出码非 0（Rust run_repair 以退出码为准，旧版吞错致假成功）；
 * - 写入前先备份（同名 .bak——已存在则沿用，不覆盖更早的备份）；修复期间
 *   检测到文件被其他进程写入（活跃会话）时保留现场并报告失败。
 *
 * 用法:
 *   node scripts/repair-session.mjs <sessionId 或 session.jsonl(.zstd) 路径>
 *   node scripts/repair-session.mjs --all  # 扫描并修复 $DSH_HOME/sessions/ 下所有会话
 */

import { existsSync, readFileSync, writeFileSync, copyFileSync, readdirSync, statSync, renameSync, unlinkSync } from 'node:fs'
import { join } from 'node:path'
import { homedir } from 'node:os'
import { pathToFileURL } from 'node:url'
import { zstdCompress, zstdDecompressSync, constants } from 'node:zlib'
import { promisify } from 'node:util'

const zstdCompressAsync = promisify(zstdCompress)
const CHECKSUM_OPTIONS = { params: { [constants.ZSTD_c_checksumFlag]: 1 } }
const ZSTD_MAGIC = 4247762216

function getDshHome() {
  return process.env.DSH_HOME || join(homedir(), '.dsh')
}

/**
 * 引擎存活标志（Rust 侧注入 DSH_ENGINE_ALIVE=1/0）：以壳持有的 dsh 子进程
 * try_wait 为准（WSL 客体形态下 wsl.exe 子进程即会话存活代理）。
 * 「运行中」复合判据的前半（2026-09-07）：dsh 未运行时任何会话文件都不可能
 * 再被写入——mtime 新鲜不再构成「运行中」。旧版裸 mtime 判据会把归档/重命名
 * （均会刷新 mtime）误标成「运行中」5 分钟。
 */
function engineAlive() {
  return process.env.DSH_ENGINE_ALIVE === '1'
}

/** 与 dsh 加载器 scanZstdFrames 语义一致的结构扫描。 */
function scanZstdFrames(buffer) {
  const frames = []
  let offset = 0
  while (offset < buffer.length) {
    const start = offset
    if (buffer.length - offset < 4) return { frames, tornStart: start }
    if (buffer.readUInt32LE(offset) !== ZSTD_MAGIC) return { frames, tornStart: start }
    offset += 4
    const descriptor = buffer.readUInt8(offset)
    offset += 1
    const contentSizeFlag = descriptor >>> 6
    const singleSegment = (descriptor & 32) !== 0
    const dictionaryFlag = descriptor & 3
    const dictionaryBytes = dictionaryFlag === 3 ? 4 : dictionaryFlag
    const contentSizeBytes = contentSizeFlag === 0 ? (singleSegment ? 1 : 0) : 1 << contentSizeFlag
    offset += (singleSegment ? 0 : 1) + dictionaryBytes + contentSizeBytes
    for (;;) {
      if (buffer.length - offset < 3) return { frames, tornStart: start }
      const blockHeader = buffer.readUIntLE(offset, 3)
      offset += 3
      const lastBlock = (blockHeader & 1) !== 0
      const blockType = (blockHeader >>> 1) & 3
      const blockSize = blockHeader >>> 3
      if (blockType === 3) return { frames, tornStart: start }
      const payloadBytes = blockType === 1 ? 1 : blockSize
      if (buffer.length - offset < payloadBytes) return { frames, tornStart: start }
      offset += payloadBytes
      if (lastBlock) break
    }
    if ((descriptor & 4) !== 0) offset += 4
    frames.push({ start, end: offset })
  }
  return { frames }
}

/** 解压全部完整帧为明文（加载器语义：torn 尾帧由加载器单独恢复，此处只在全帧 OK 时使用）。 */
function decompressZstd(buffer) {
  const { frames, tornStart } = scanZstdFrames(buffer)
  if (frames.length === 0) throw new Error('no complete zstd frames')
  if (tornStart !== undefined) throw new Error(`torn zstd frame at byte ${tornStart}`)
  return Buffer.concat(frames.map((f) => zstdDecompressSync(buffer.subarray(f.start, f.end))))
}

/** 与 dsh 写入器同构：首帧恰好一行 header，事件帧与 header 帧分离、带校验和。 */
async function compressZstdFrames(headerLine, eventLines) {
  const headerBuf = Buffer.from(headerLine + '\n', 'utf8')
  const eventsBuf = Buffer.from(eventLines.join('\n') + '\n', 'utf8')
  const headerFrame = await zstdCompressAsync(headerBuf, CHECKSUM_OPTIONS)
  const eventsFrame = await zstdCompressAsync(eventsBuf, CHECKSUM_OPTIONS)
  return Buffer.concat([headerFrame, eventsFrame])
}

// ---------------------------------------------------------------------------
// 引擎档 dsh-session 本尊解析（2026-09-07）：恢复层校验优先用 dsh 自己的代码，
// 与实际加载该会话的 dsh 同版本 → 校验结论零漂移。找不到时返回 null（调用方
// 降级 fallback fold，不静默回到「只看 seq 连续」）。
// ---------------------------------------------------------------------------

/** 语义化版本比较（足够分辨 0.1.2-rc.1 vs 0.1.2 vs 0.1.10；同版本号 release > 预发布）。 */
function compareVersions(a, b) {
  const parse = (v) => {
    const [core, pre] = v.split('-')
    const nums = core.split('.').map((n) => parseInt(n, 10) || 0)
    return { nums, pre: pre ?? null }
  }
  const pa = parse(a)
  const pb = parse(b)
  for (let i = 0; i < 3; i++) {
    if ((pa.nums[i] || 0) !== (pb.nums[i] || 0)) return (pa.nums[i] || 0) - (pb.nums[i] || 0)
  }
  if (pa.pre === null && pb.pre !== null) return 1
  if (pa.pre !== null && pb.pre === null) return -1
  if (pa.pre === null && pb.pre === null) return 0
  return pa.pre < pb.pre ? -1 : pa.pre > pb.pre ? 1 : 0
}

/** 在引擎档 global 树的 .pnpm 目录里找 @deepseek-ai/<name>，取最高版本。 */
function findEnginePackageSync(packageName) {
  const engines = process.env.DSH_DOCK_ENGINES
  if (!engines) return null
  const globalV11 = join(engines, 'global', 'v11')
  let entries
  try {
    entries = readdirSync(globalV11)
  } catch {
    return null
  }
  const dirName = `@deepseek-ai+${packageName}@`
  const candidates = []
  for (const entry of entries) {
    const pnpmDir = join(globalV11, entry, 'node_modules', '.pnpm')
    let names
    try {
      names = readdirSync(pnpmDir)
    } catch {
      continue
    }
    for (const name of names) {
      if (!name.startsWith(dirName)) continue
      const version = name.slice(dirName.length).split('_')[0]
      candidates.push({ version, dir: join(pnpmDir, name, 'node_modules', '@deepseek-ai', packageName) })
    }
  }
  if (candidates.length === 0) return null
  candidates.sort((a, b) => compareVersions(b.version, a.version))
  return candidates[0]
}

let dshValidator = undefined // undefined=未解析 null=已解析但不可用 {dsh,catalog,version,hasCatalog}

/**
 * 解析引擎档校验器（进程内缓存）：
 * - dsh-session 本尊（必需）：Session 恢复链；
 * - dsh-session-format-catalog（可选，dsh ≥0.1.3 才存在）：世代格式解码/迁移/
 *   编码管线。currentVersion 必须与 dsh-session 的 SESSION_FORMAT_VERSION
 *   一致（同代安装；不一致视为不可用，走无迁移路径）。
 * 返回 null 表示不可用——调用方必须走 fallback，且不得把 fallback 结论冒充 dsh 结论。
 */
async function getDshValidator() {
  if (dshValidator !== undefined) return dshValidator
  dshValidator = null
  const foundSession = findEnginePackageSync('dsh-session')
  if (!foundSession) return dshValidator
  try {
    const mod = await import(pathToFileURL(join(foundSession.dir, 'lib', 'index.js')).href)
    for (const key of ['Session', 'SESSION_FORMAT_VERSION', 'KNOWN_SESSION_EVENT_TYPES', 'decodeSeqRanges', 'decodeStorageRecord', 'interruptedTurnClosers']) {
      if (mod[key] === undefined) throw new Error(`export missing: ${key}`)
    }
    const entry = { dsh: mod, version: foundSession.version, catalog: null, hasCatalog: false }
    const foundCatalog = findEnginePackageSync('dsh-session-format-catalog')
    if (foundCatalog) {
      try {
        const catalogMod = await import(pathToFileURL(join(foundCatalog.dir, 'lib', 'index.js')).href)
        const catalog = catalogMod.sessionFormatCatalog
        if (catalog && catalog.currentVersion === mod.SESSION_FORMAT_VERSION) {
          entry.catalog = catalog
          entry.hasCatalog = true
        }
      } catch { /* catalog 缺失/不兼容：无迁移路径 */ }
    }
    dshValidator = entry
  } catch {
    dshValidator = null
  }
  return dshValidator
}

// ---------------------------------------------------------------------------
// 存储层（行级）分析：行展开、出处链存储形、重叠/缺口。手写、锚加载器语义。
// ---------------------------------------------------------------------------

/** 会话头合法即通过（对齐 dsh 加载器 parseHeaderRecord 的 isHeaderLine 全量检查）。 */
function isSessionHeader(value) {
  return (
    typeof value === 'object' && value !== null && value.type === 'session' &&
    typeof value.version === 'number' && typeof value.id === 'string' &&
    typeof value.createdAt === 'number' && Number.isSafeInteger(value.createdAt) && value.createdAt >= 0 && !Object.is(value.createdAt, -0) &&
    typeof value.delegationDepth === 'number' && Number.isSafeInteger(value.delegationDepth) && value.delegationDepth >= 0 && !Object.is(value.delegationDepth, -0) &&
    (value.seedLength === undefined || (typeof value.seedLength === 'number' && Number.isSafeInteger(value.seedLength) && value.seedLength >= 0 && !Object.is(value.seedLength, -0))) &&
    (value.origin === undefined || value.origin === 'subagent') &&
    (value.agentPreset === undefined || typeof value.agentPreset === 'string')
  )
}

/**
 * 校验 sourceEventSeqs 出处链（对齐 dsh 加载器 expandProvenanceFromStorage /
 * decodeSeqRanges 语义）：
 * - 纯数字列表**允许乱序**（dsh 语义：数字条目本身不要求递增——加载器只
 *   在出现 [start,end] 区间时才强制整体严格递增）；
 * - 含区间对 [start,end] 时：start <= end 且展开后**整体**严格递增；
 * - 条目总数不得超过 record.seq（maxEntries 上限）——违反 = 出处链损坏
 *   （2026-09-05 实测：旧版重编号后 sourceEventSeqs 未同步，展开条目数
 *   远超新 seq，dsh 打开即 `unparsable committed event`）。
 */
function checkSourceEventSeqs(record) {
  const raw = record.sourceEventSeqs
  if (raw === undefined) return
  if (!Array.isArray(raw)) throw new Error('sourceEventSeqs must be an array')
  const maxEntries = Number.isSafeInteger(record.seq) && record.seq >= 0 ? record.seq : Number.MAX_SAFE_INTEGER
  let count = 0
  let hasRange = false
  for (const entry of raw) {
    if (typeof entry === 'number') {
      if (!Number.isSafeInteger(entry) || entry < 0) throw new Error('sourceEventSeqs must contain non-negative safe integers')
      count += 1
    } else if (Array.isArray(entry) && entry.length === 2) {
      const [start, end] = entry
      if (!Number.isSafeInteger(start) || start < 0 || !Number.isSafeInteger(end) || end < start) {
        throw new Error('sourceEventSeqs ranges require valid start <= end')
      }
      hasRange = true
      count += end - start + 1
    } else {
      throw new Error('sourceEventSeqs entries must be numbers or [start, end] pairs')
    }
  }
  if (count > maxEntries) {
    throw new Error(`sourceEventSeqs exceeds its event sequence (expanded ${count} > seq ${maxEntries})`)
  }
  if (hasRange) {
    // 仅在含区间对时，要求展开后的整体序列严格递增（与 decodeSeqRanges 一致）
    let prev = -1
    for (const entry of raw) {
      if (typeof entry === 'number') {
        if (entry <= prev) throw new Error('sourceEventSeqs must be strictly increasing')
        prev = entry
      } else {
        const [start, end] = entry
        if (start <= prev) throw new Error('sourceEventSeqs must be strictly increasing')
        prev = end
      }
    }
  }
}

/**
 * 计算一条存储记录展开后的事件 seq 区间 [lo, hi]。
 * 与 dsh-session chunk-rows validateRow/expandRow 的判据对齐（envelope 精确键、
 * payload 为字符串数组、dt 为安全整数且长度 = payload-1）。
 * 区间之外的语义细节（dt 时间演进安全界等）由恢复层校验兜底。
 */
function expandSpan(record) {
  const tag = record?.type
  const isRow = tag === 'text-chunks' || tag === 'reasoning-chunks' || tag === 'tool-call-chunks'
  if (!isRow) {
    const seq = record?.seq
    if (typeof seq !== 'number') return null // 无 seq 事件：加载器将其视作截断点（缺失类）
    checkSourceEventSeqs(record)
    return { lo: seq, hi: seq }
  }

  const envKeys = Object.keys(record).sort().join(',')
  if (envKeys !== 'data,seq0,time0,type') throw new Error(`malformed ${tag} storage row: envelope must be exactly {type, seq0, time0, data}`)
  if (!Number.isSafeInteger(record.seq0) || record.seq0 < 0) throw new Error(`malformed ${tag} storage row: seq0 must be a non-negative safe integer`)
  if (!Number.isSafeInteger(record.time0)) throw new Error(`malformed ${tag} storage row: time0 must be a safe integer`)
  const data = record.data
  if (data === null || typeof data !== 'object' || Array.isArray(data)) throw new Error(`malformed ${tag} storage row: data must be an object`)

  const payloadKey = tag === 'tool-call-chunks' ? 'args' : 'texts'
  const expectedKeys =
    tag === 'tool-call-chunks'
      ? ['args', 'dt', 'id', 'index', 'name', 'step', 'turn'].sort().join(',')
      : ['dt', 'index', 'step', 'texts', 'turn'].sort().join(',')
  const actualKeys = Object.keys(data).sort().join(',')
  if (
    actualKeys !== expectedKeys &&
    !(tag === 'tool-call-chunks' && actualKeys === ['args', 'dt', 'id', 'index', 'step', 'turn'].sort().join(','))
  ) {
    throw new Error(`malformed ${tag} storage row: data must be exactly {turn, step, index${tag === 'tool-call-chunks' ? ', id, name?' : ''}, dt, ${payloadKey}}`)
  }

  const payload = data[payloadKey]
  if (!Array.isArray(payload) || payload.length === 0 || payload.some((e) => typeof e !== 'string')) {
    throw new Error(`malformed ${tag} storage row: ${payloadKey} must be a non-empty string array`)
  }
  const dt = data.dt
  if (!Array.isArray(dt) || dt.some((g) => !Number.isSafeInteger(g)) || dt.length !== payload.length - 1) {
    throw new Error(`malformed ${tag} storage row: dt must be ${payload.length - 1} safe integers`)
  }
  if (!Number.isSafeInteger(record.seq0 + payload.length - 1)) {
    throw new Error(`malformed ${tag} storage row: member seqs must stay safe integers`)
  }
  return { lo: record.seq0, hi: record.seq0 + payload.length - 1 }
}

/** 加载器语义校验：展开后 seq 从 0 严格连续（header 单独提供）。 */
function verifyLoaderSemantics(headerLine, records) {
  if (!isSessionHeader(JSON.parse(headerLine))) return 'bad header'
  let next = 0
  for (const record of records) {
    let span
    try {
      span = expandSpan(record)
    } catch (e) {
      return `decode error: ${e.message}`
    }
    if (span === null || span.lo !== next) return `seq gap at ${next}`
    next = span.hi + 1
  }
  return null
}

/**
 * 单轮分析：返回 { kind, records?, detail }。
 * 与 dsh 加载器 consumeEventLine 的判定一致：
 * - span.lo === next：接受，推进；
 * - span.lo > next：缺口（加载器截断到连续前缀）；
 * - span.lo < next：重放重叠（中断恢复后 dsh 自 S 起以相同 seq 重写）。
 */
function analyzeOnce(records) {
  let next = 0
  for (let i = 0; i < records.length; i++) {
    let span
    try {
      span = expandSpan(records[i])
    } catch (e) {
      return { kind: 'unrepairable', detail: `第 ${i + 2} 行行语义损坏：${e.message}` }
    }
    if (span === null) {
      return { kind: 'truncated', records: records.slice(0, i), detail: `第 ${i + 2} 行起无 seq（加载器视作截断点），已截断到连续前缀` }
    }
    if (span.lo < next) {
      // 重放重叠：重放块自 S 起；旧记录（含被遮蔽的占位事件）是前缀内
      // 第一个 hi >= S 的项开始的连续区段。优先保留最新重放块。
      const S = span.lo
      let c = -1
      for (let k = 0; k < i; k++) {
        const sk = expandSpan(records[k])
        if (sk !== null && sk.hi >= S) {
          c = k
          break
        }
      }
      if (c === -1) return { kind: 'unrepairable', detail: `第 ${i + 2} 行起 seq 倒退到 ${S}，但前缀无对应占位区间` }
      const sc = expandSpan(records[c])
      if (sc.lo !== S) {
        return { kind: 'unrepairable', detail: `重放起点 ${S} 落在第 ${c + 2} 行的打包区间内部（${sc.lo}–${sc.hi}），无法无损分离` }
      }
      const kept = records.slice(0, c).concat(records.slice(i))
      return {
        kind: 'repaired',
        records: kept,
        detail: `重放重叠已修复：丢弃被遮蔽的旧事件（第 ${c + 2}–${i + 1} 行，seq ${S} 起的旧占位/旧尾部），保留重放块及其后全部事件，顺序与 seq 原样`,
      }
    }
    if (span.lo > next) {
      return {
        kind: 'truncated',
        records: records.slice(0, i),
        detail: `第 ${i + 2} 行 seq=${span.lo} 跳变（期望 ${next}），已按加载器语义截断到连续前缀`,
      }
    }
    next = span.hi + 1
  }
  return { kind: 'healthy' }
}

/** 迭代修复：一次修复可能暴露更深一层的重叠（多次中断恢复），逐轮收敛。
 * 任何一轮都没有实际变更时返回 healthy（避免对健康文件重写/备份）。 */
function analyzeAndRepair(records) {
  const MAX_ROUNDS = 16
  let current = records
  let applied = false
  let firstDetail = ''
  for (let round = 0; round < MAX_ROUNDS; round++) {
    const res = analyzeOnce(current)
    if (res.kind === 'repaired') {
      current = res.records
      applied = true
      if (!firstDetail) firstDetail = res.detail
      continue
    }
    if (res.kind === 'healthy') {
      if (!applied) return { kind: 'healthy' }
      return { kind: 'repaired', records: current, detail: firstDetail }
    }
    if (res.kind === 'truncated') {
      // 截断发生在重放块内部：后续事件不可达，保留当前连续前缀（加载器同款语义）。
      if (applied) {
        return { kind: 'repaired', records: res.records, detail: `${firstDetail}\n${res.detail}` }
      }
      return res
    }
    return res
  }
  return { kind: 'unrepairable', detail: '重放重叠嵌套过深，无法安全收敛' }
}

// ---------------------------------------------------------------------------
// 恢复层校验（2026-09-07）：dsh 本尊 Session.fromRestore 优先，fallback fold 兜底。
// 统一入口 validateRecords(header, records) → null | Error。records 为存储行
// （未展开），两模式内部各自展开。
// ---------------------------------------------------------------------------

/** 由存储 header 行构造逻辑 SessionHeader（对齐 fromHeaderLine）。 */
function logicalMetaFromHeader(header) {
  if (Object.hasOwn(header, 'sandboxMode') || Object.hasOwn(header, 'approvalPolicy')) {
    throw new Error('session header uses retired policy baseline fields')
  }
  return {
    version: header.version,
    id: header.id,
    createdAt: header.createdAt,
    ...(header.cwd !== undefined ? { cwd: header.cwd } : {}),
    ...(header.parentSession !== undefined ? { parentSession: header.parentSession } : {}),
    isSeeded: header.seedLength !== undefined,
    ...(header.origin !== undefined ? { origin: header.origin } : {}),
    delegationDepth: header.delegationDepth,
    ...(header.agentPreset !== undefined ? { agentPreset: header.agentPreset } : {}),
  }
}

/**
 * 加载器扫描复刻（consumeEventLine 语义）：存储行 → 事件流。含出处链解码、
 * 延迟 issue 语义（缺口在下一个 turn/end 才抛）。返回 { events, issue }。
 */
function scanRecordsToEvents(dsh, records) {
  const events = []
  let issue
  for (let i = 0; i < records.length; i++) {
    let decoded
    try {
      let parsed = records[i]
      if (parsed.sourceEventSeqs !== undefined) {
        parsed = { ...parsed, sourceEventSeqs: dsh.decodeSeqRanges(parsed.sourceEventSeqs, parsed.seq) }
      }
      decoded = dsh.decodeStorageRecord(parsed)
    } catch {
      issue ??= new Error(`corrupt session log: unparsable committed event at record ${i + 2}`)
      continue
    }
    if (issue !== undefined) {
      if (decoded.some((e) => e.type === 'turn/end')) return { events, issue }
      continue
    }
    const rowStart = events.length
    let gapInRow = false
    for (const event of decoded) {
      if (event.seq !== events.length) {
        events.length = rowStart
        issue = new Error(`corrupt session log: seq gap in committed region at record ${i + 2} (expected ${events.length}, got ${event.seq})`)
        gapInRow = true
        break
      }
      events.push(event)
    }
    if (gapInRow && decoded.some((c) => c.type === 'turn/end')) return { events, issue }
  }
  return { events, issue }
}

/**
 * prepareCore 恢复链：词汇表闸门 → adoptSessionEvent → interruptedTurnClosers
 * 补尾 → Session.fromRestore（含 surface fold 全量重放）。
 * 成功返回 null；失败返回携带 dsh 原始报错的 Error。
 */
function prepareCoreChain(dsh, meta, inheritedEventCount, events) {
  for (const event of events) {
    if (!dsh.KNOWN_SESSION_EVENT_TYPES.has(event.type) && event.ignorable !== true) {
      return new Error(`session contains event type "${event.type}" (seq ${event.seq}) unknown to this harness and not marked ignorable`)
    }
    try {
      dsh.adoptSessionEvent(event)
    } catch (e) {
      return e instanceof Error ? e : new Error(String(e))
    }
  }
  try {
    const closers = dsh.interruptedTurnClosers(events)
    dsh.Session.fromRestore(meta.id, structuredClone([...events, ...closers]), meta, inheritedEventCount)
    return null
  } catch (e) {
    return e instanceof Error ? e : new Error(String(e))
  }
}

/** 版本不被本构建支持的错误（比对「已装 dsh 更新的存储格式」与「迁移器拒绝」
 * 都归此类）：不可修复、不可变异，健康分类应为 unknown + 升级提示，而非
 * needs_repair。 */
function unsupportedVersionError(message) {
  const err = new Error(message)
  err.unsupportedVersion = true
  return err
}

/**
 * dsh 无 catalog 模式（0.1.2 代）：存储版本必须与已装 dsh 完全一致，走直接
 * 恢复链。catalog 存在时的版本路由在 makeRestoreValidator 里。
 */
function restoreThroughDshLegacy(dsh, header, records) {
  if (header.version !== dsh.SESSION_FORMAT_VERSION) {
    return unsupportedVersionError(
      `stored Session format is v${header.version}; installed dsh writes v${dsh.SESSION_FORMAT_VERSION}`,
    )
  }
  let meta
  try {
    meta = logicalMetaFromHeader(header)
  } catch (e) {
    return e instanceof Error ? e : new Error(String(e))
  }
  const { events, issue } = scanRecordsToEvents(dsh, records)
  if (issue !== undefined) return issue // 存储层健康时不可达；防御性兜底（宁可报修不可放行）
  return prepareCoreChain(dsh, meta, header.seedLength ?? 0, events)
}

/**
 * dsh + catalog 模式（0.1.3+）：复刻真实读路径——readHeader 版本分类 →
 * decodeRecoverableArtifact → migrate → encodeCurrent（世代发布语义）→
 * 用迁移后的 v2 表示走与 0.1.2 相同的扫描 + 恢复链。迁移内部本就含
 * restoreCurrent 的 fromRestore 全量校验；末段再用扫描+恢复链兜一道。
 * 注意 v2 物理行与 v0 不同（一行一事件、header 直接带 isSeeded），存储层
 * v0 锚定代码不适用于迁移产物——所以必须在迁移后表示上判定。
 */
function restoreThroughDshCatalog(entry, header, records) {
  const { dsh, catalog } = entry
  const classified = catalog.readHeader(header)
  if (classified.status === 'unsupported') {
    return unsupportedVersionError(classified.reason || 'stored Session format is not supported by this build')
  }
  if (classified.status === 'malformed') {
    return new Error(`corrupt session log: ${classified.reason || 'malformed header'}`)
  }
  let migrated
  try {
    const decoded = catalog.decodeRecoverableArtifact(header, records)
    migrated = catalog.migrate(decoded)
  } catch (e) {
    // SessionFormatUnsupportedMigrationError = 迁移器拒绝（与「更新版本」同类，
    // 不可修复）；其余按损坏处理（可修复性由修复循环裁定）。
    if (e && (e.name === 'SessionFormatUnsupportedMigrationError' || String(e.constructor?.name ?? '') === 'SessionFormatUnsupportedMigrationError')) {
      return unsupportedVersionError(e.message)
    }
    return e instanceof Error ? e : new Error(String(e))
  }
  const encoded = catalog.encodeCurrent(migrated)
  const { events, issue } = scanRecordsToEvents(dsh, encoded.rows)
  if (issue !== undefined) return issue
  return prepareCoreChain(dsh, migrated.header, migrated.inheritedEventCount ?? 0, events)
}

// ----- fallback：surface fold 移植（锚 dsh v0.1.2-rc.1
// packages/core/session/src/{index,surface}.ts，2026-09-07 对照；引擎档缺包时
// 才使用。行集展开只看 span，不手写 chunk 行解码——fold 对非 surface 事件仅
// 要求 seq 合法，行内成员 seq 由 expandSpan 保证。） -----

const SURFACE_EVENT_TYPES = new Set(['user/message', 'assistant/message', 'tool/result'])

function isEventSeq(value) {
  return typeof value === 'number' && Number.isSafeInteger(value) && value >= 0 && !Object.is(value, -0)
}

function isReplaceOp(value) {
  const op = value
  return Object.keys(op).length === 3
    && Object.hasOwn(op, 'op') && Object.hasOwn(op, 'start') && Object.hasOwn(op, 'end')
    && op.op === 'replace' && isEventSeq(op.start) && isEventSeq(op.end)
}

/** 事件局部 surface 合法性（对齐 surfaceOpOf）。 */
function surfaceOpOf(record) {
  if (!SURFACE_EVENT_TYPES.has(record.type)) {
    if (record.surfaceOp !== undefined) throw new Error(`session event "${record.type}" is not surface-eligible and cannot carry surfaceOp`)
    if (record.sourceEventSeqs !== undefined) throw new Error(`session event "${record.type}" is not surface-eligible and cannot carry sourceEventSeqs`)
    return undefined
  }
  const op = record.surfaceOp
  if (op === undefined) throw new Error(`session event "${record.type}" is surface-eligible and requires a surfaceOp marker`)
  if (op === 'append') return op
  if (op === null || typeof op !== 'object' || Array.isArray(op)) {
    throw new Error(`session event "${record.type}" carries an invalid surfaceOp`)
  }
  if (!isReplaceOp(op)) throw new Error(`session event "${record.type}" carries an invalid replace surfaceOp`)
  return op
}

/**
 * 出处链存储形 → 内存形（对齐已安装 dsh v0.1.2-rc.1 decodeSeqRanges：数字与
 * [start,end] 对混排、maxEntries 上限、含区间时展开整体严格递增）。
 */
function decodeSeqRangesFallback(value, maxEntries) {
  if (!Array.isArray(value)) throw new TypeError('sourceEventSeqs must be an array')
  const decoded = []
  let hasRange = false
  const assertSeq = (v) => {
    if (!Number.isSafeInteger(v) || v < 0) throw new TypeError('sourceEventSeqs must contain non-negative safe integers')
  }
  for (const entry of value) {
    if (typeof entry === 'number') {
      assertSeq(entry)
      if (decoded.length >= maxEntries) throw new TypeError('sourceEventSeqs exceeds its event sequence')
      decoded.push(entry)
      continue
    }
    if (!Array.isArray(entry) || entry.length !== 2) throw new TypeError('sourceEventSeqs range entries must be [start, end] pairs')
    const [start, end] = entry
    assertSeq(start)
    assertSeq(end)
    if (end < start) throw new TypeError('sourceEventSeqs ranges require start <= end')
    if (end - start + 1 > maxEntries - decoded.length) throw new TypeError('sourceEventSeqs range exceeds its event sequence')
    for (let seq = start; seq <= end; seq += 1) decoded.push(seq)
    hasRange = true
  }
  if (hasRange) {
    for (let i = 1; i < decoded.length; i++) {
      if (decoded[i] <= decoded[i - 1]) throw new TypeError('sourceEventSeqs ranges must be strictly increasing')
    }
  }
  return decoded
}

/** 对齐 v0.1.2-rc.1 assertProvenance（注意：该版本允许 assistant/message 携带
 * 出处链，仅空数组在非 assistant 类型上非法——勿以仓库 HEAD 0.1.3 的规则为准）。 */
function assertProvenance(event, shadowedSeqs) {
  const raw = event.sourceEventSeqs
  const sources = new Set()
  if (raw !== undefined) {
    if (!Array.isArray(raw)) throw new Error(`sourceEventSeqs on event at seq ${event.seq} must be an array when present`)
    if (raw.length === 0 && event.type !== 'assistant/message') throw new Error('sourceEventSeqs must not be empty except on assistant/message')
    let nonEarlierSource
    for (const source of raw) {
      if (!isEventSeq(source)) throw new Error(`session event "${event.type}" sourceEventSeqs must densely contain non-negative safe integers`)
      sources.add(source)
      if (nonEarlierSource === undefined && source >= event.seq) nonEarlierSource = source
    }
    if (sources.size !== raw.length) throw new Error('sourceEventSeqs must not contain duplicates')
    if (nonEarlierSource !== undefined) throw new Error(`sourceEventSeqs must reference earlier events: ${nonEarlierSource} >= current seq ${event.seq}`)
  }
  const missing = shadowedSeqs.filter((seq) => !sources.has(seq))
  if (missing.length > 0) {
    throw new Error(`surface replace: sourceEventSeqs must include every shadowed surface node; missing ${missing.join(', ')}`)
  }
}

function isDeepEqualJson(a, b) {
  if (a === b) return true
  if (Array.isArray(a) || Array.isArray(b)) {
    if (!Array.isArray(a) || !Array.isArray(b) || a.length !== b.length) return false
    return a.every((item, i) => isDeepEqualJson(item, b[i]))
  }
  if (typeof a !== 'object' || typeof b !== 'object' || a === null || b === null) return false
  const aKeys = Object.keys(a)
  if (aKeys.length !== Object.keys(b).length) return false
  return aKeys.every((key) => Object.hasOwn(b, key) && isDeepEqualJson(a[key], b[key]))
}

/** 对齐 assertToolResultRewrite：tool/result 替换只允许改内容、只允许一个当前节点。 */
function assertToolResultRewrite(record, shadowedSeqs, seqToRecord) {
  if (record.type !== 'tool/result') return
  if (shadowedSeqs.length !== 1) throw new Error('tool/result surface replacement must rewrite exactly one current node')
  for (const originalSeq of shadowedSeqs) {
    const original = seqToRecord.get(originalSeq)
    if (original?.type !== 'tool/result') throw new Error('tool/result surface replacement must target a current tool/result')
    const originalRest = { ...original.data }
    const replacementRest = { ...record.data }
    const originalResult = original.data.message.content[0]
    const replacementResult = record.data.message.content[0]
    originalRest.message = { ...original.data.message, content: [{ ...originalResult, content: null }] }
    replacementRest.message = { ...record.data.message, content: [{ ...replacementResult, content: null }] }
    if (!isDeepEqualJson(originalRest, replacementRest)) {
      throw new Error('tool/result surface replacement may change only content')
    }
  }
}

// ----- fallback：事件 envelope 形状（锚 index.ts assertSessionEventEnvelope /
// assertCurrentLlmShape / assertMessageEventShape，2026-09-07 对照） -----

function hasProviderModel(value) {
  return typeof value === 'object' && value !== null
    && typeof value.provider === 'string' && value.provider.length > 0
    && typeof value.model === 'string' && value.model.length > 0
}

function assertMessageEventShape(event, subject) {
  const type = event.type
  if (type !== 'user/message' && type !== 'assistant/message' && type !== 'tool/result') return
  const data = event.data
  const record = typeof data === 'object' && data !== null ? data : undefined
  const message = type === 'user/message' ? record : record?.message
  if (typeof message !== 'object' || message === null || typeof message.id !== 'string' || message.id === '') {
    throw new Error(`${subject} lacks an identified message`)
  }
  const expectedRole = type === 'assistant/message' ? 'assistant' : 'user'
  if (message.role !== expectedRole) throw new Error(`${subject} message must have role "${expectedRole}"`)
  const source = message.source
  if (typeof source !== 'object' || source === null || typeof source.kind !== 'string' || source.kind === '') {
    throw new Error(`${subject} message has invalid source`)
  }
  if (!Array.isArray(message.content)) throw new Error(`${subject} message has invalid content`)
  if (type === 'assistant/message') {
    if (source.kind !== 'model' || !hasProviderModel(source)) throw new Error(`${subject} message must have model source`)
    return
  }
  if (type !== 'tool/result') return
  if (source.kind !== 'tool' || typeof source.callId !== 'string' || source.callId === '') {
    throw new Error(`${subject} message must have tool source`)
  }
  const block = message.content[0]
  if (message.content.length !== 1 || typeof block !== 'object' || block === null || block.type !== 'tool-result' || !Array.isArray(block.content)) {
    throw new Error(`${subject} message must contain one tool-result block`)
  }
  if (block.toolCallId !== source.callId) throw new Error(`${subject} message has mismatched tool call ids`)
}

function assertEnvelope(event, index) {
  if (event.type === 'request/header-delta') throw new Error(`seed event at index ${index} uses unsupported legacy request/header-delta format`)
  for (const key in event) {
    switch (key) {
      case 'type': case 'seq': case 'time': case 'data':
      case 'surfaceOp': case 'sourceEventSeqs': case 'ignorable': break
      default: throw new Error(`seed event at index ${index} has an invalid event envelope`)
    }
  }
  if (
    typeof event.type !== 'string' || !isEventSeq(event.seq) || !Number.isSafeInteger(event.time) ||
    event.data === undefined || (event.ignorable !== undefined && event.ignorable !== true)
  ) throw new Error(`seed event at index ${index} has an invalid event envelope`)
  if (event.type === 'request/header') {
    const header = event.data?.header
    if (!hasProviderModel(header?.config)) throw new Error(`seed request/header at index ${index} lacks provider/model`)
  }
  assertMessageEventShape(event, `seed ${event.type} at index ${index}`)
}

/**
 * fallback 校验：行级连续（expandSpan）+ envelope + fold 全量重放。
 * 与 dsh 模式的差异：不做 interruptedTurnClosers 补尾（裸前缀 fold 严格于
 * dsh 的平衡后校验，方向保守——不会漏报会话损坏，可能对中断尾多报一次可修）。
 */
function restoreViaFallback(header, records) {
  // fallback 锚定 v0 存储格式（无迁移管线）：其他版本一律「不支持」而非损坏
  // ——真实判定必须由引擎档 dsh 本尊做出。
  if (header.version !== 0) {
    throw unsupportedVersionError(
      `stored Session format is v${header.version}; 内置 fallback 校验仅支持 v0（需引擎档 dsh 本尊判定）`,
    )
  }
  const meta = logicalMetaFromHeader(header)
  const state = { nodes: [], generation: 0 }
  const seqToRecord = new Map()
  let next = 0
  for (let i = 0; i < records.length; i++) {
    const record = records[i]
    const span = expandSpan(record)
    if (span === null) throw new Error(`record ${i + 2} has no seq (loader treats as truncation point)`)
    if (span.lo !== next) throw new Error(`seq gap at ${next} (record ${i + 2})`)
    const isRow = record.type === 'text-chunks' || record.type === 'reasoning-chunks' || record.type === 'tool-call-chunks'
    if (!isRow) {
      // 出处链先按加载器语义解码到内存形再做 envelope/fold 校验（存储形允许
      // [start,end] 区间，校验只见数字）。
      let event = record
      if (record.sourceEventSeqs !== undefined) {
        try {
          event = { ...record, sourceEventSeqs: decodeSeqRangesFallback(record.sourceEventSeqs, isEventSeq(record.seq) ? record.seq : Number.MAX_SAFE_INTEGER) }
        } catch (e) {
          throw new Error(`unparsable committed event (record ${i + 2}): ${e.message}`)
        }
      }
      assertEnvelope(event, span.lo)
      const expectedSeq = span.lo
      const surfaceOp = surfaceOpOf(event)
      if (surfaceOp !== undefined) {
        if (surfaceOp === 'append') {
          assertProvenance(event, [])
          state.nodes.push(event.seq)
        } else {
          const startIdx = state.nodes.indexOf(surfaceOp.start)
          if (startIdx === -1) throw new Error(`surface replace: start seq ${surfaceOp.start} not found in surface`)
          const endIdx = state.nodes.indexOf(surfaceOp.end)
          if (endIdx === -1) throw new Error(`surface replace: end seq ${surfaceOp.end} not found in surface`)
          if (startIdx > endIdx) throw new Error(`surface replace: start seq ${surfaceOp.start} (index ${startIdx}) is after end seq ${surfaceOp.end} (index ${endIdx})`)
          const shadowedSeqs = state.nodes.slice(startIdx, endIdx + 1)
          assertProvenance(event, shadowedSeqs)
          assertToolResultRewrite(event, shadowedSeqs, seqToRecord)
          state.nodes.splice(startIdx, endIdx - startIdx + 1, event.seq)
          state.generation += 1
        }
      }
      seqToRecord.set(record.seq, record)
    }
    next = span.hi + 1
  }
  return null
}

/**
 * 统一恢复层入口：header + 存储行 → null（可加载）| Error（dsh 打开必失败，
 * message 与 dsh 原始报错一致或为 fallback 移植语义）。Error 带有
 * unsupportedVersion 标记时表示「本构建不支持的格式版本」，不可修复。
 */
function makeRestoreValidator(entry) {
  if (entry) {
    if (entry.hasCatalog) {
      return (header, records) => {
        try {
          return restoreThroughDshCatalog(entry, header, records)
        } catch (e) {
          return e instanceof Error ? e : new Error(String(e))
        }
      }
    }
    return (header, records) => restoreThroughDshLegacy(entry.dsh, header, records)
  }
  return (header, records) => {
    try {
      return restoreViaFallback(header, records)
    } catch (e) {
      return e instanceof Error ? e : new Error(String(e))
    }
  }
}

// ---------------------------------------------------------------------------
// surface 级修复（第 4 类）：二分定位首个坏记录 → 最小变异 → 全量复验。
// 绝不改 seq、不删事件、不重排——只动 surfaceOp / sourceEventSeqs 两个元数据
// 字段（消息内容原样）。
// ---------------------------------------------------------------------------

/** 单记录最小变异：返回 { record, desc } | null（不可变异）。 */
function mutateRecord(record) {
  const eligible = SURFACE_EVENT_TYPES.has(record?.type)
  if (eligible) {
    const op = record.surfaceOp
    if (op !== undefined && op !== 'append') {
      // 悬空/畸形 replace（含 start/end 引用不存在节点）：转 append 保留消息，
      // 并剥离失效出处链（其引用的被遮蔽节点已不可达）。
      const { sourceEventSeqs, ...rest } = record
      return {
        record: { ...rest, surfaceOp: 'append' },
        desc: `seq ${record.seq} 的 replace surfaceOp 引用已不存在的 surface 节点，已转为 append（消息内容与 seq 原样保留，仅失去遮蔽语义）`,
      }
    }
    if (op === undefined) {
      return {
        record: { ...record, surfaceOp: 'append' },
        desc: `seq ${record.seq} 缺少 surfaceOp 标记，已补 append`,
      }
    }
    if ('sourceEventSeqs' in record) {
      const { sourceEventSeqs, ...rest } = record
      return {
        record: rest,
        desc: `seq ${record.seq} 的 sourceEventSeqs 出处链失效，已剥离该元数据（内容与 seq 原样保留）`,
      }
    }
    return null
  }
  if (record && typeof record === 'object' && ('surfaceOp' in record || 'sourceEventSeqs' in record)) {
    const { surfaceOp, sourceEventSeqs, ...rest } = record
    return {
      record: rest,
      desc: `非 surface 事件（${record.type ?? '?'}）携带了 surface 元数据字段，已剥离`,
    }
  }
  return null
}

/**
 * surface 修复循环：反复「全量校验 → 二分找首个坏记录 → 最小变异」，直至
 * 通过或放弃。返回 { ok, records, changes, detail }。
 */
function repairSurfaceLoop(records, validate, maxRounds = 32) {
  let current = records
  const changes = []
  for (let round = 0; round < maxRounds; round++) {
    const err = validate(current)
    if (!err) return { ok: true, records: current, changes, detail: changes.join('；') }
    // 二分定位：validate(prefix) 首个失败长度 = 首个坏记录位置
    let lo = 1
    let hi = current.length
    let bad = current.length
    while (lo <= hi) {
      const mid = (lo + hi) >> 1
      if (validate(current.slice(0, mid))) {
        bad = mid
        hi = mid - 1
      } else {
        lo = mid + 1
      }
    }
    const target = current[bad - 1]
    const mutated = mutateRecord(target)
    if (!mutated) {
      return { ok: false, records: current, changes, detail: `恢复校验失败且不可安全变异（seq ${target?.seq}）：${err.message}` }
    }
    changes.push(mutated.desc)
    current = [...current.slice(0, bad - 1), mutated.record, ...current.slice(bad)]
  }
  return { ok: false, records: current, changes, detail: `surface 损坏嵌套过深（>${maxRounds} 轮），无法安全收敛` }
}

/** 稳定读取：stat 前后一致（避免读到写一半的文件），最多重试 3 次。 */
function readStable(filePath) {
  for (let attempt = 0; attempt < 3; attempt++) {
    const before = statSync(filePath)
    const buffer = readFileSync(filePath)
    const after = statSync(filePath)
    if (before.size === after.size && before.mtimeMs === after.mtimeMs && before.ino === after.ino) {
      return { buffer, before }
    }
  }
  throw new Error('文件在读取期间持续被写入（活跃会话），无法稳定读取')
}

/** 统一健康结果形态：早退路径无 header/不可解析时元数据取缺省值。 */
function healthOut(status, extra = {}) {
  return {
    status,
    title: '',
    eventCount: 0,
    active: false,
    createdAt: null,
    endState: null,
    subagent: false,
    agentPreset: null,
    detail: '',
    ...extra,
  }
}

/**
 * 只读健康检查（不写回、不备份）：
 * status ∈ healthy | needs_repair | unknown。
 * - healthy：存储层 seq 连续 **且** 恢复层（dsh fromRestore / fallback fold）通过；
 * - needs_repair：存储层存在可安全修复的重放重叠/缺口，或恢复层失败但属
 *   可最小变异的 surface/出处链损坏；
 * - unknown：无法解析/不可安全修复/读取失败。
 * 元数据字段（2026-09-07 扩展项1，均信息性、不参与健康判定）：
 * - title：最新 `session/title` 事件（跳过 sourceEventSeqs 镜像行）；
 * - createdAt：header.createdAt（毫秒 epoch，2026-09-07 实证真实日志量级）；
 * - eventCount：展开后事件总数（chunk 行按 payload 展开；损坏行不计入）；
 * - endState：最后一条 `turn/end` 的 reason.kind（stop/interrupted…），
 *   无 turn/end = 'open'（未正常收尾，常见于崩溃尾）；不可解析为 null；
 * - subagent / agentPreset：header.origin === 'subagent' / header.agentPreset；
 * - active（运行中）：复合判据（2026-09-07 立法；2026-09-08 降噪增订）=
 *   引擎存活（engineAlive()，Rust 注入 DSH_ENGINE_ALIVE）**且** mtime 距今
 *   <5 分钟（dsh 批量写间隔分钟级，可能仍 flush）**且** endState === 'open'
 *   （正常收尾过的会话即使 mtime 新鲜也不再标运行中——排除「刚结束但末次
 *   flush 还在窗口内」的假阳性；已知代价：活跃会话两轮对话间隙末条也是
 *   stop，存在漏标，裁定接受，见问题记录095 #5）。仅作 UI 徽标与修复
 *   预拦，不参与健康判定。
 */
export function scanSessionHealth(filePath) {
  const isZstd = filePath.endsWith('.zstd')
  let buffer
  let st = null
  try {
    const stable = readStable(filePath)
    buffer = stable.buffer
    st = stable.before
  } catch (e) {
    return healthOut('unknown', { detail: `读取失败：${e.message}` })
  }
  if (buffer.length === 0) {
    return healthOut('unknown', { detail: '文件为空' })
  }

  // 活跃标志前半（引擎存活 + mtime 新鲜）：复合判据见函数头注释。终值在
  // endState 解析后合成（末尾 active = activeBase && endState === 'open'）。
  const activeBase = engineAlive() && st !== null && Date.now() - st.mtimeMs < 5 * 60 * 1000

  let headerLine
  let records
  let header = null
  try {
    const rawText = isZstd ? decompressZstd(buffer).toString('utf8') : buffer.toString('utf8')
    const lines = rawText.split('\n').map((l) => l.trim()).filter(Boolean)
    if (lines.length === 0) {
      return healthOut('unknown', { active: activeBase, detail: '文件为空' })
    }
    headerLine = lines[0]
    try {
      header = JSON.parse(headerLine)
    } catch {
      return healthOut('unknown', { active: activeBase, detail: 'header 非法' })
    }
    if (!isSessionHeader(header)) {
      return healthOut('unknown', { active: activeBase, detail: 'header 非法' })
    }
    records = []
    for (let i = 1; i < lines.length; i++) {
      try {
        records.push(JSON.parse(lines[i]))
      } catch (e) {
        return healthOut('unknown', { active: activeBase, detail: `第 ${i + 1} 行 JSON 解析失败` })
      }
    }
  } catch (e) {
    return healthOut('unknown', { active: activeBase, detail: `解压失败：${e.message}` })
  }

  // 元数据透出（2026-09-07）：header 派生字段。
  const createdAt = typeof header.createdAt === 'number' ? header.createdAt : null
  const subagent = header.origin === 'subagent'
  const agentPreset = typeof header.agentPreset === 'string' ? header.agentPreset : null

  // 事件数：展开后的事件总数（chunk 行按 payload 长度展开）。损坏行不计入
  //（健康状态另行判定），此处仅信息性统计。
  let eventCount = 0
  for (const r of records) {
    try {
      const span = expandSpan(r)
      if (span) eventCount += span.hi - span.lo + 1
    } catch { /* 损坏行不计入 */ }
  }

  // 标题提取：会话标题以**最新** `session/title` 事件为准（dsh 标题体系：
  // 首条 user 消息生成初稿，LLM/分析器随后生成最终标题——projcache 记录
  // 的是最终值；取首个会与 dsh Web 侧边栏不一致，2026-09-05 实测 8650）。
  // 跳过 sourceEventSeqs 修饰的镜像行（重复的检索快照）。
  let title = ''
  for (const r of records) {
    if (r?.type === 'session/title' && !('sourceEventSeqs' in r)) {
      const t = r?.data?.title
      if (typeof t === 'string' && t.trim()) {
        title = t.trim() // 持续覆盖 → 最终取到最后一个
      }
    }
  }

  // 结束状态：最后一条 turn/end 的 reason.kind（最后者胜，跳过镜像行）；
  // 无 turn/end = 'open'（未正常收尾）。
  let endState = 'open'
  for (const r of records) {
    if (r?.type === 'turn/end' && !('sourceEventSeqs' in r)) {
      const kind = r?.data?.reason?.kind
      if (typeof kind === 'string' && kind) endState = kind
    }
  }

  // 活跃终值：正常收尾（turn/end 存在）的会话不再标运行中（降噪，2026-09-08）。
  const active = activeBase && endState === 'open'

  const meta = { title, eventCount, active, createdAt, endState, subagent, agentPreset }

  // 第一层（存储层）：与修复分析同一套判定
  const decision = analyzeAndRepair(records)
  if (decision.kind === 'unrepairable') {
    return healthOut('unknown', { ...meta, detail: decision.detail })
  }
  if (decision.kind !== 'healthy') {
    return healthOut('needs_repair', { ...meta, detail: decision.detail })
  }

  // 第二层（恢复层）：dsh 打开会话的确切路径（或 fallback）
  const validate = makeRestoreValidator(dshValidator)
  const restoreErr = validate(header, records)
  if (!restoreErr) {
    return healthOut('healthy', meta)
  }
  if (restoreErr.unsupportedVersion) {
    // 本构建不支持的格式版本（存储比已装 dsh 新 / 迁移器拒绝 / fallback 无迁移
    // 管线）：不是损坏，不可修复——归类 unknown 并给升级提示，避免误导用户点修复。
    return healthOut('unknown', { ...meta, detail: `存储格式版本不受支持（不可修复，需升级适配）：${restoreErr.message}` })
  }
  return healthOut('needs_repair', {
    ...meta,
    detail: `恢复校验失败（dsh 打开将报错）：${restoreErr.message}`,
  })
}

/** 同步休眠（Node 主线程允许 Atomics.wait；退避重试用）。 */
function sleepSync(ms) {
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, ms)
}

/**
 * Windows 韧性 rename（2026-09-07 维护者裁定：重试+退避）：Windows 上
 * rename 不能替换仍被任何进程打开的文件（EPERM -4048，杀软/索引器/备份
 * 工具瞬态锁）——退避重试 3 次（200/500/1000ms）穷尽才上抛；非 EPERM
 * 原样抛。失败无损：原文件未动，tmp 留修好内容可重试。
 */
function renameWithRetry(tmp, dest) {
  const backoffs = [200, 500, 1000]
  for (let attempt = 0; ; attempt++) {
    try {
      renameSync(tmp, dest)
      return
    } catch (e) {
      if (e?.code !== 'EPERM' || attempt >= backoffs.length) throw e
      sleepSync(backoffs[attempt])
    }
  }
}

/** 原子替换：写临时文件 → 覆盖（Windows 先删目标，POSIX rename 原子）。 */
function writeAtomic(filePath, bytes) {
  const tmp = `${filePath}.dsh-repair-tmp`
  writeFileSync(tmp, bytes)
  try {
    renameSync(tmp, filePath)
  } catch (e) {
    try {
      unlinkSync(filePath)
    } catch { /* 目标不存在也允许 */ }
    renameSync(tmp, filePath)
  }
}

/**
 * 修复单个会话文件。
 * 返回 { ok, changed, message }；ok=false 时文件已被回滚为原样（或未动）。
 */
export async function repairSessionFile(filePath) {
  if (!existsSync(filePath)) {
    return { ok: false, changed: false, message: `❌ 文件不存在: ${filePath}` }
  }

  // 活跃会话语义（2026-09-05 修订；2026-09-07 复合判据前置见 scanSessionHealth）：
  // dsh 对会话文件的写入是**间歇性 flush**（实测 828cfec4 分钟级间隔、间隙长达
  // 数分钟），按 mtime 阈值预判会误杀大量可修窗口。正确判定 = 读取稳定性
  //（readStable 的 stat 前后一致性）+ 写后 stat 复查（下方已实现）；读取期间
  // 持续写入才会被拒绝。
  // 残余竞态评估（2026-09-07，锚引擎档 dsh-session-persistence-jsonl@0.1.2-rc.1
  // lib/index.js appendLines）：dsh 追加为逐批 open("a") → write → fsync →
  // close，不持长驻 fd；失败批次回滚到旧尺寸且游标不变（下一批重试）。故修复
  // 与写入竞争的最坏情形 = dsh 稍后重放其事件批次，不会永久丢数据。

  const isZstd = filePath.endsWith('.zstd')
  let original
  try {
    original = readStable(filePath)
  } catch (e) {
    return { ok: false, changed: false, message: `❌ ${e.message}（会话可能仍在持续写入中，请稍后重试）: ${filePath}` }
  }

  if (original.before.size === 0) {
    return { ok: false, changed: false, message: `⚠️ 文件为空，跳过: ${filePath}` }
  }

  let headerLine
  let records
  try {
    const rawText = isZstd ? decompressZstd(original.buffer).toString('utf8') : original.buffer.toString('utf8')
    const lines = rawText.split('\n').map((l) => l.trim()).filter(Boolean)
    if (lines.length === 0) {
      return { ok: false, changed: false, message: `⚠️ 文件为空，跳过: ${filePath}` }
    }
    headerLine = lines[0]
    const header = JSON.parse(headerLine)
    if (!isSessionHeader(header)) {
      return { ok: false, changed: false, message: `❌ 无法解析 Session Header: ${filePath}` }
    }
    records = []
    for (let i = 1; i < lines.length; i++) {
      try {
        records.push(JSON.parse(lines[i]))
      } catch (e) {
        return {
          ok: false,
          changed: false,
          message: `❌ 第 ${i + 1} 行 JSON 解析失败（不可安全修复，文件未动）: ${e.message}`,
        }
      }
    }
  } catch (e) {
    return { ok: false, changed: false, message: `❌ ${e.message}` }
  }

  const sessionId = JSON.parse(headerLine).id
  const validate = makeRestoreValidator(dshValidator)
  const validatorName = dshValidator
    ? `dsh-session@${dshValidator.version}（引擎档本尊${dshValidator.hasCatalog ? ` + format-catalog v${dshValidator.catalog.currentVersion}` : ''}）`
    : '内置 fold（引擎档缺包降级）'

  // 第一层：存储层分析（重叠去重 / 缺口截断）
  let decision = analyzeAndRepair(records)

  if (decision.kind === 'unrepairable') {
    // 第 4 类前置（2026-09-05 实测）：sourceEventSeqs 出处链损坏（旧版重编号
    // 遗留——seq 已连续但 sourceEventSeqs 指向旧 seq，展开条目数超限/乱序，
    // dsh expandProvenanceFromStorage 抛 `unparsable committed event`）。
    // 剥离该字段可让记录原样通过加载器（仅失去溯源元数据，不影响内容）。
    const stripped = records.map((r) => {
      if (r && 'sourceEventSeqs' in r) {
        const { sourceEventSeqs, ...rest } = r
        return rest
      }
      return r
    })
    const retry = analyzeAndRepair(stripped)
    if (retry.kind === 'healthy' || retry.kind === 'repaired' || retry.kind === 'truncated') {
      decision = {
        kind: retry.kind,
        records: retry.records ?? stripped,
        detail: `sourceEventSeqs 出处链损坏（可能为旧版重编号遗留），已剥离该元数据字段（内容与 seq 原样保留）：${decision.detail}`,
      }
    } else {
      return { ok: false, changed: false, message: `❌ 会话 ${sessionId} 无法安全修复：${decision.detail}` }
    }
  }

  let candidate = decision.records ?? records
  const details = decision.kind === 'healthy' ? [] : [decision.detail]

  // 第二层：恢复层校验 + surface 级最小变异修复（2026-09-07，第 4 类）。
  // unsupportedVersion（存储版本不受本构建支持）不可修复，直接如实报告。
  const firstRestoreErr = validate(JSON.parse(headerLine), candidate)
  if (firstRestoreErr?.unsupportedVersion) {
    return {
      ok: false,
      changed: false,
      message: `❌ 会话 ${sessionId} 存储格式版本不受本构建支持，无法修复（文件未动）：${firstRestoreErr.message}`,
    }
  }
  if (decision.kind === 'healthy') {
    if (firstRestoreErr) {
      const loop = repairSurfaceLoop(candidate, (rs) => validate(JSON.parse(headerLine), rs))
      if (!loop.ok) {
        return {
          ok: false,
          changed: false,
          message: `❌ 会话 ${sessionId} 存储层正常但 dsh 恢复校验失败，且无法安全修复（文件未动）\n  校验器：${validatorName}\n  错误：${firstRestoreErr.message}\n  ${loop.detail}`,
        }
      }
      candidate = loop.records
      details.push(`恢复层修复（校验器 ${validatorName}）：${loop.detail}`)
    }
  } else {
    // 存储层动过（截断/去重）之后同样必须过恢复层——两层都绿才允许写盘。
    if (firstRestoreErr) {
      const loop = repairSurfaceLoop(candidate, (rs) => validate(JSON.parse(headerLine), rs))
      if (!loop.ok) {
        return {
          ok: false,
          changed: false,
          message: `❌ 会话 ${sessionId} 存储层修复后仍未通过 dsh 恢复校验，已放弃写入（原文件未动）：${firstRestoreErr.message}`,
        }
      }
      candidate = loop.records
      details.push(`恢复层修复（校验器 ${validatorName}）：${loop.detail}`)
    }
  }

  // 无任何变更 = 幂等 no-op：不写回、不备份。
  const changedCount = candidate.length === records.length
    ? candidate.filter((r, i) => r !== records[i]).length
    : candidate.length // 截断/去重必然变了行数
  if (changedCount === 0 && decision.kind === 'healthy') {
    return { ok: true, changed: false, message: `✅ 会话 ${sessionId} 恢复校验通过（${validatorName}），无需修复。` }
  }

  // 修复路径：备份 → 写回 → 存储层+恢复层双校验 → 失败放弃（不触原文件）。
  const backupPath = `${filePath}.bak`
  if (!existsSync(backupPath)) {
    try {
      copyFileSync(filePath, backupPath)
    } catch (e) {
      return { ok: false, changed: false, message: `❌ 创建备份失败（已取消修复）: ${e.message}` }
    }
  }

  // 重放重叠应保留 header 原样（含 seedLength 语义），截断同理。
  const keptHeaderLine = headerLine
  const eventLines = candidate.map((r) => JSON.stringify(r))
  const repairedBytes = isZstd
    ? await compressZstdFrames(keptHeaderLine, eventLines)
    : Buffer.from(`${keptHeaderLine}\n${eventLines.join('\n')}\n`, 'utf8')

  // 先写临时文件验证，再原子替换（验证失败不用动正式文件）。
  const tmpPath = `${filePath}.dsh-repair-tmp`
  writeFileSync(tmpPath, repairedBytes)
  let verifyErr = null
  try {
    const checkBuf = isZstd ? decompressZstd(readFileSync(tmpPath)) : readFileSync(tmpPath)
    const checkText = checkBuf.toString('utf8')
    const checkLines = checkText.split('\n').map((l) => l.trim()).filter(Boolean)
    verifyErr = verifyLoaderSemantics(checkLines[0], checkLines.slice(1).map((l) => JSON.parse(l)))
    if (verifyErr === null) {
      // 恢复层闸门：对**写盘字节**再跑一次 dsh 恢复链（防序列化回归）。
      const gateErr = validate(JSON.parse(checkLines[0]), checkLines.slice(1).map((l) => JSON.parse(l)))
      if (gateErr) verifyErr = `restore gate: ${gateErr.message}`
    }
  } catch (e) {
    verifyErr = e.message
  }
  if (verifyErr !== null) {
    try {
      unlinkSync(tmpPath)
    } catch { /* ignore */ }
    return { ok: false, changed: false, message: `❌ 修复产物未通过加载器语义校验，已放弃写入（原文件未动）：${verifyErr}` }
  }
  renameWithRetry(tmpPath, filePath)

  // 写后竞态复查：确认磁盘上的文件正是本次写入的字节（而非期间被其他进程
  // 改写——活跃会话会在修复期间继续追加）。
  // 策略（2026-09-05 实测脆断后修订）：竞态 = 立即停止并返回失败，**保留现场**
  // （不回滚到原始损坏数据——回滚会破坏已生效的修复，随后重试又可能与写入
  // 竞争，最终文件仍是损坏态但脚本报告成功，即用户看到的「点了修复没修复」）。
  // 双重视角：① 磁盘字节与写入字节比对；② stat 修订（size+mtime+ino）与
  // 写入前后比对——文件系统的 stat 比内容读更有机会发现并发写入。
  let diskBytes
  let diskStat
  try {
    diskBytes = readFileSync(filePath)
    diskStat = statSync(filePath)
  } catch {
    diskBytes = null
    diskStat = null
  }
  const wroteStat = (() => {
    try {
      return statSync(filePath)
    } catch {
      return null
    }
  })()
  const statChanged =
    diskStat && wroteStat &&
    (diskStat.size !== wroteStat.size ||
      diskStat.mtimeMs !== wroteStat.mtimeMs ||
      diskStat.ino !== wroteStat.ino)
  if ((diskBytes && !diskBytes.equals(repairedBytes)) || statChanged) {
    return {
      ok: false,
      changed: false,
      message: `⚠️ 会话 ${sessionId} 修复期间文件被其他进程写入（可能为活跃会话），本次未生效；文件已保留为修复后的状态，请稍后在会话静止时再次修复。`,
    }
  }

  return {
    ok: true,
    changed: true,
    message: `✨ 会话 ${sessionId} 已修复（${isZstd ? 'zstd' : 'jsonl'}，校验器 ${validatorName}）。${details.join('；')}`,
  }
}

/** 会话日志文件名（含 0.1.3+ 世代文件族）：session.jsonl[.zstd]（v0 源世代）
 * 与 session.vN.jsonl[.zstd]（v1+ 不可变世代）。 */
const SESSION_LOG_FILENAME = /^session(?:\.v\d+)?\.jsonl(?:\.zstd)?$/

async function findSessionFiles(rootDir) {
  const sessionFiles = []
  function walk(dir) {
    if (!existsSync(dir)) return
    const entries = readdirSync(dir, { withFileTypes: true })
    for (const ent of entries) {
      const full = join(dir, ent.name)
      if (ent.isDirectory()) {
        walk(full)
      } else if (ent.isFile() && SESSION_LOG_FILENAME.test(ent.name)) {
        sessionFiles.push(full)
      }
    }
  }
  walk(rootDir)
  return sessionFiles
}

async function main() {
  const args = process.argv.slice(2)
  if (args.length === 0 || args.includes('-h') || args.includes('--help')) {
    console.log(`
DSH Session Repair Tool (dsh-dock 自愈工具)
-------------------------------------------
用法:
  node scripts/repair-session.mjs <sessionId 或 路径>
  node scripts/repair-session.mjs --all
  node scripts/repair-session.mjs --scan   # 只读扫描，输出 JSON（供壳端健康检查与标题提取）

示例:
  node scripts/repair-session.mjs session-af85a2e7-7c2e-44c5-a498-afeb3ba79297
  node scripts/repair-session.mjs ~/.dsh/sessions/--my-project--/session-xxx/session.jsonl.zstd
  node scripts/repair-session.mjs --all
  node scripts/repair-session.mjs --scan
`)
    process.exit(0)
  }

  // 恢复层校验器先解析（引擎档 dsh-session [+ format-catalog] 或 fallback），全程复用。
  dshValidator = await getDshValidator()

  const dshHome = getDshHome()
  const sessionsDir = join(dshHome, 'sessions')

  if (args[0] === '--scan') {
    const files = await findSessionFiles(sessionsDir)
    const out = files.map((file) => {
      const health = scanSessionHealth(file)
      return {
        path: file,
        status: health.status,
        title: health.title || null,
        active: health.active,
        detail: health.detail || null,
        validator: dshValidator ? `dsh-session@${dshValidator.version}${dshValidator.hasCatalog ? '+catalog' : ''}` : 'fallback',
        createdAt: health.createdAt,
        eventCount: health.eventCount,
        endState: health.endState,
        subagent: health.subagent,
        agentPreset: health.agentPreset,
      }
    })
    console.log(JSON.stringify(out))
    process.exit(0)
  }

  if (args[0] === '--all') {
    console.log(`🚀 开始全量扫描会话目录: ${sessionsDir}`)
    const files = await findSessionFiles(sessionsDir)
    console.log(`共发现 ${files.length} 个会话日志文件。`)
    let failed = 0
    for (const file of files) {
      // 活跃会话（引擎存活 + mtime < 5 分钟 + 未正常收尾，复合判据）跳过：
      // 修复必然被下次 flush 覆盖（假成功）；dsh 未运行时不可能有写入，
      // mtime 不参与判定。全量修复只处理静止/已结束的会话。
      const s = scanSessionHealth(file)
      if (s.active) {
        console.log(`⏭️  跳过活跃会话 ${file.split(/[\\/]/).filter(Boolean).slice(-2, -1)[0] || ''}（dsh 运行中且近期有写入，结束后可修复）。`)
        continue
      }
      const res = await repairSessionFile(file)
      console.log(res.message)
      if (!res.ok) failed++
    }
    console.log(`\n${failed === 0 ? '🎉' : '❗'} 全量检查与修复完成：${files.length - failed}/${files.length} 成功。`)
    process.exit(failed === 0 ? 0 : 1)
  }

  const target = args[0]
  let targetPath = target

  if (!existsSync(targetPath)) {
    const files = await findSessionFiles(sessionsDir)
    const matched = files.find((f) => f.includes(target))
    if (matched) {
      targetPath = matched
    } else {
      console.error(`❌ 未找到匹配的会话文件: ${target}`)
      process.exit(1)
    }
  }

  // 单文件修复：活跃会话（引擎存活 + mtime < 5 分钟 + 未正常收尾，复合判据）
  // 明确提示——修复会被下次 flush 覆盖；dsh 未运行时 mtime 新鲜不再构成活跃。
  const health = scanSessionHealth(targetPath)
  if (health.active) {
    console.log(`⏭️  会话仍被 dsh 使用（运行中且近期有写入、未见正常收尾）。为避免修复被下一次写入覆盖，请稍后在会话结束后再修复。`)
    process.exit(1)
  }

  const res = await repairSessionFile(targetPath)
  console.log(res.message)
  process.exit(res.ok ? 0 : 1)
}

main().catch((err) => {
  console.error('执行失败:', err)
  process.exit(1)
})
