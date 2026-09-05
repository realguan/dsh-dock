#!/usr/bin/env node
/**
 * @file repair-session.mjs
 * @description DSH 会话自愈与修复工具（2026-09-04 重写：与上游加载器语义对齐）。
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
 *
 * 安全约束：
 * - 修复输出必须通过「加载器语义校验」：展开 chunk 行后 seq 严格连续且从 0 起。
 * - 校验失败 → 用内存中的原始字节回滚，退出码非 0。
 * - 无需修复（健康）→ 文件保持原样（不写回），退出码 0。
 * - 任何失败路径退出码非 0（Rust run_repair 以退出码为准，旧版吞错致假成功）。
 * - 写入前先备份（同名 .bak ——与 dsh 加载器/旧版约定一致）；修复期间检测到
 *   文件被其他进程写入（活跃会话）时重试一次，仍竞态则报告失败不写坏文件。
 *
 * 用法:
 *   node scripts/repair-session.mjs <sessionId 或 session.jsonl(.zstd) 路径>
 *   node scripts/repair-session.mjs --all  # 扫描并修复 $DSH_HOME/sessions/ 下所有会话
 */

import { existsSync, readFileSync, writeFileSync, copyFileSync, readdirSync, statSync, renameSync, unlinkSync } from 'node:fs'
import { join } from 'node:path'
import { homedir } from 'node:os'
import { zstdCompress, zstdDecompressSync, constants } from 'node:zlib'
import { promisify } from 'node:util'

const zstdCompressAsync = promisify(zstdCompress)
const CHECKSUM_OPTIONS = { params: { [constants.ZSTD_c_checksumFlag]: 1 } }
const ZSTD_MAGIC = 4247762216

function getDshHome() {
  return process.env.DSH_HOME || join(homedir(), '.dsh')
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

/** 会话头合法即通过（加载器 parseHeaderRecord 的 isHeaderLine 子集检查）。 */
function isSessionHeader(value) {
  return (
    typeof value === 'object' && value !== null && value.type === 'session' &&
    typeof value.version === 'number' && typeof value.id === 'string' &&
    typeof value.createdAt === 'number' && Number.isSafeInteger(value.createdAt) &&
    typeof value.delegationDepth === 'number' && Number.isSafeInteger(value.delegationDepth)
  )
}

/**
 * 计算一条存储记录展开后的事件 seq 区间 [lo, hi]。
 * 与 dsh-session chunk-rows validateRow/expandRow 的判据对齐（envelope 精确键、
 * payload 为字符串数组、dt 为安全整数且长度 = payload-1）。
 * 区间之外的语义细节（dt 时间演进安全界等）由写后校验兜底。
 */
function expandSpan(record) {
  const tag = record?.type
  const isRow = tag === 'text-chunks' || tag === 'reasoning-chunks' || tag === 'tool-call-chunks'
  if (!isRow) {
    const seq = record?.seq
    if (typeof seq !== 'number') return null // 无 seq 事件：加载器将其视作截断点（缺失类）
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

/**
 * 只读健康检查（不写回、不备份）：返回 { status, title, eventCount, detail }。
 * status ∈ healthy | needs_repair | unknown。
 * - healthy：seq 严格连续（加载器语义）；
 * - needs_repair：存在可安全修复的重放重叠/缺口（分析器判定 repaired/truncated）；
 * - unknown：无法解析/不可安全修复（unrepairable）/读取失败。
 * 标题取自 `session/title` 事件（dsh 会话摘要，含 sourceEventSeqs 行不取，
 * 取首个非 sourceEventSeqs 的 title 事件）。
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
    return { status: 'unknown', title: '', eventCount: 0, active: false, detail: `读取失败：${e.message}` }
  }
  if (buffer.length === 0) {
    return { status: 'unknown', title: '', eventCount: 0, active: false, detail: '文件为空' }
  }

  // 活跃标志（仅 UI 提示，不参与健康判定）：mtime 距今 <5 分钟视为可能仍在
  // 被 dsh 间歇 flush（dsh 批量写间隔分钟级）。用于前端显示「运行中」徽标，
  // 与「需自愈」区分——活跃会话不能修（修了会被下次 flush 覆盖）。
  const active = st !== null && Date.now() - st.mtimeMs < 5 * 60 * 1000

  let headerLine
  let records
  try {
    const rawText = isZstd ? decompressZstd(buffer).toString('utf8') : buffer.toString('utf8')
    const lines = rawText.split('\n').map((l) => l.trim()).filter(Boolean)
    if (lines.length === 0) {
      return { status: 'unknown', title: '', eventCount: 0, active, detail: '文件为空' }
    }
    headerLine = lines[0]
    if (!isSessionHeader(JSON.parse(headerLine))) {
      return { status: 'unknown', title: '', eventCount: 0, active, detail: 'header 非法' }
    }
    records = []
    for (let i = 1; i < lines.length; i++) {
      try {
        records.push(JSON.parse(lines[i]))
      } catch (e) {
        return { status: 'unknown', title: '', eventCount: 0, active, detail: `第 ${i + 1} 行 JSON 解析失败` }
      }
    }
  } catch (e) {
    return { status: 'unknown', title: '', eventCount: 0, active, detail: `解压失败：${e.message}` }
  }

  // 标题提取：首个 session/title 事件（跳过 sourceEventSeqs 修饰的镜像行）
  let title = ''
  for (const r of records) {
    if (r?.type === 'session/title' && !('sourceEventSeqs' in r)) {
      const t = r?.data?.title
      if (typeof t === 'string' && t.trim()) {
        title = t.trim()
        break
      }
    }
  }

  // 健康判定：与修复分析同一套判定
  const decision = analyzeAndRepair(records)
  if (decision.kind === 'healthy') {
    return { status: 'healthy', title, eventCount: 0, active, detail: '' }
  }
  if (decision.kind === 'repaired' || decision.kind === 'truncated') {
    return { status: 'needs_repair', title, eventCount: 0, active, detail: decision.detail }
  }
  return { status: 'unknown', title, eventCount: 0, active, detail: decision.detail }
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

  // 活跃会话语义（2026-09-05 修订）：dsh 对会话文件的写入是**间歇性 flush**
  // （实测 828cfec4 分钟级间隔、间隙长达数分钟），按 mtime 阈值预判会误杀
  // 大量可修窗口（用户点击时恰逢上次 flush 不久 → 永远"活跃"→ 永远修不了）。
  // 正确判定 = 读取稳定性（readStable 的 stat 前后一致性）+ 写后 stat 复查
  // （下方已实现）；读取期间持续写入才会被拒绝。

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
  const decision = analyzeAndRepair(records)

  if (decision.kind === 'healthy') {
    return { ok: true, changed: false, message: `✅ 会话 ${sessionId} 序列正常，无需修复。` }
  }

  if (decision.kind === 'unrepairable') {
    return { ok: false, changed: false, message: `❌ 会话 ${sessionId} 无法安全修复：${decision.detail}` }
  }

  // 修复路径：备份 → 写回 → 加载器语义校验 → 失败回滚。
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
  const eventLines = decision.records.map((r) => JSON.stringify(r))
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
  } catch (e) {
    verifyErr = e.message
  }
  if (verifyErr !== null) {
    try {
      unlinkSync(tmpPath)
    } catch { /* ignore */ }
    return { ok: false, changed: false, message: `❌ 修复产物未通过加载器语义校验，已放弃写入（原文件未动）：${verifyErr}` }
  }
  renameSync(tmpPath, filePath)

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
    message: `✨ 会话 ${sessionId} 已修复（${isZstd ? 'zstd' : 'jsonl'}）。${decision.detail}`,
  }
}

async function findSessionFiles(rootDir) {
  const sessionFiles = []
  function walk(dir) {
    if (!existsSync(dir)) return
    const entries = readdirSync(dir, { withFileTypes: true })
    for (const ent of entries) {
      const full = join(dir, ent.name)
      if (ent.isDirectory()) {
        walk(full)
      } else if (ent.isFile() && (ent.name === 'session.jsonl.zstd' || ent.name === 'session.jsonl')) {
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
      // 活跃会话（mtime < 5 分钟）跳过：修复必然被下次 flush 覆盖（假成功），
      // 全量修复只处理静止/已结束的会话。
      const s = scanSessionHealth(file)
      if (s.active) {
        console.log(`⏭️  跳过活跃会话 ${file.split(/[\\/]/).filter(Boolean).slice(-2, -1)[0] || ''}（仍在运行，结束后可修复）。`)
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

  // 单文件修复：活跃会话（mtime < 5 分钟）明确提示——修复会被下次 flush 覆盖。
  const health = scanSessionHealth(targetPath)
  if (health.active) {
    console.log(`⏭️  会话仍被 dsh 使用（活跃，mtime 距今 <5 分钟）。为避免修复被下一次写入覆盖，请稍后在会话结束后再修复。`)
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
