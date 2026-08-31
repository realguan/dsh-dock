#!/usr/bin/env node
/**
 * @file repair-session.mjs
 * @description DSH 会话自愈与修复工具。
 * 针对上游 dsh 会话日志中因断线重连、并发推流或旧轮次延迟落盘导致的
 * "seq gap in committed region" / 乱序交叉写入等问题进行自动检测、备份与修复。
 *
 * 用法:
 *   node scripts/repair-session.mjs <sessionId 或 session.jsonl.zstd 路径>
 *   node scripts/repair-session.mjs --all  # 扫描并修复 ~/.dsh/sessions/ 下的所有会话
 */

import { existsSync, readFileSync, writeFileSync, copyFileSync, readdirSync } from 'node:fs'
import { join } from 'node:path'
import { homedir } from 'node:os'
import { execSync } from 'node:child_process'
import { promisify } from 'node:util'
import { zstdCompress, constants } from 'node:zlib'

const zstdCompressAsync = promisify(zstdCompress)
const CHECKSUM_OPTIONS = {
  params: { [constants.ZSTD_c_checksumFlag]: 1 },
}

function getDshHome() {
  return process.env.DSH_HOME || join(homedir(), '.dsh')
}

function decompressZstd(filePath) {
  try {
    return execSync(`zstd -dc "${filePath}"`, { maxBuffer: 100 * 1024 * 1024 })
  } catch (e) {
    throw new Error(`zstd 解压失败 (${filePath}): ${e.message}`)
  }
}

async function compressZstdFrames(headerLine, eventsLines) {
  const headerBuf = Buffer.from(headerLine + '\n', 'utf8')
  const headerFrame = await zstdCompressAsync(headerBuf, CHECKSUM_OPTIONS)

  const eventsBuf = Buffer.from(eventsLines + '\n', 'utf8')
  const eventsFrame = await zstdCompressAsync(eventsBuf, CHECKSUM_OPTIONS)

  return Buffer.concat([headerFrame, eventsFrame])
}

export async function repairSessionFile(filePath) {
  console.log(`\n🔍 正在检查会话文件: ${filePath}`)
  if (!existsSync(filePath)) {
    console.error(`❌ 文件不存在: ${filePath}`)
    return false
  }

  const isZstd = filePath.endsWith('.zstd')
  let rawText = ''
  if (isZstd) {
    rawText = decompressZstd(filePath).toString('utf8')
  } else {
    rawText = readFileSync(filePath, 'utf8')
  }

  const lines = rawText.split('\n').map(l => l.trim()).filter(Boolean)
  if (lines.length === 0) {
    console.warn(`⚠️ 文件为空，跳过: ${filePath}`)
    return false
  }

  let header
  try {
    header = JSON.parse(lines[0])
  } catch (e) {
    console.error(`❌ 无法解析 Session Header: ${e.message}`)
    return false
  }

  console.log(`   Session ID: ${header.id}`)
  console.log(`   总记录行数: ${lines.length}`)

  // 解析所有事件行
  const records = []
  for (let i = 1; i < lines.length; i++) {
    try {
      const parsed = JSON.parse(lines[i])
      records.push({ lineIndex: i + 1, raw: parsed })
    } catch (e) {
      console.error(`❌ 第 ${i + 1} 行 JSON 解析失败: ${e.message}`)
    }
  }

  // 检查是否存在序列异常（非连续、倒退、交叉轮次）
  let hasAnomalies = false
  let lastTurn = null
  let maxTurnSeen = -1

  for (let i = 0; i < records.length; i++) {
    const rec = records[i].raw
    const turn = rec.data?.turn ?? rec.turn
    if (typeof turn === 'number') {
      if (turn < maxTurnSeen && turn !== lastTurn) {
        hasAnomalies = true
      }
      if (turn > maxTurnSeen) {
        maxTurnSeen = turn
      }
      lastTurn = turn
    }
  }

  let lastSeq = -1
  for (let i = 0; i < records.length; i++) {
    const rec = records[i].raw
    const seq = rec.seq ?? rec.seq0
    if (typeof seq === 'number') {
      if (seq <= lastSeq) {
        hasAnomalies = true
      }
      lastSeq = seq
    }
  }

  if (!hasAnomalies) {
    console.log(`   ✅ 该会话序列正常，无需修复。`)
    return true
  }

  console.log(`   ⚠️ 检测到日志序列异常/多轮次交叉落盘，开始重排自愈...`)

  // 按 Turn 依赖和因果关系重新整理事件流
  const turnsMap = new Map()
  const unassigned = []

  for (const item of records) {
    const rec = item.raw
    const turn = rec.data?.turn ?? rec.turn
    if (typeof turn === 'number') {
      if (!turnsMap.has(turn)) {
        turnsMap.set(turn, [])
      }
      turnsMap.get(turn).push(rec)
    } else {
      unassigned.push(rec)
    }
  }

  // 按 Turn 序号严格单调排序
  const sortedTurns = Array.from(turnsMap.keys()).sort((a, b) => a - b)
  const orderedRecords = []

  for (const rec of unassigned) {
    orderedRecords.push(rec)
  }

  for (const t of sortedTurns) {
    const turnRecords = turnsMap.get(t)
    orderedRecords.push(...turnRecords)
  }

  // 重新为所有事件编制连续单调递增的 seq
  let currentSeq = 0
  for (const rec of orderedRecords) {
    if (rec.seq !== undefined) {
      rec.seq = currentSeq++
    } else if (rec.seq0 !== undefined) {
      rec.seq0 = currentSeq
      const count = (rec.data?.dt?.length || 0) + 1
      currentSeq += count
    } else {
      rec.seq = currentSeq++
    }
  }

  console.log(`   🔄 重新编号完成，总事件序号推进至 ${currentSeq}`)

  // 备份原文件
  const backupPath = `${filePath}.bak`
  if (!existsSync(backupPath)) {
    copyFileSync(filePath, backupPath)
    console.log(`   💾 已备份原始损坏文件至: ${backupPath}`)
  }

  // 生成修复后的内容并写入
  const headerText = JSON.stringify(header)
  const eventsText = orderedRecords.map(r => JSON.stringify(r)).join('\n')

  if (isZstd) {
    const repairedBuf = await compressZstdFrames(headerText, eventsText)
    writeFileSync(filePath, repairedBuf)
  } else {
    writeFileSync(filePath, `${headerText}\n${eventsText}\n`, 'utf8')
  }

  console.log(`   ✨ 修复完成并已保存！会话已恢复正常。`)
  return true
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

示例:
  node scripts/repair-session.mjs session-af85a2e7-7c2e-44c5-a498-afeb3ba79297
  node scripts/repair-session.mjs ~/.dsh/sessions/--my-project--/session-xxx/session.jsonl.zstd
  node scripts/repair-session.mjs --all
`)
    process.exit(0)
  }

  const dshHome = getDshHome()
  const sessionsDir = join(dshHome, 'sessions')

  if (args[0] === '--all') {
    console.log(`🚀 开始全量扫描会话目录: ${sessionsDir}`)
    const files = await findSessionFiles(sessionsDir)
    console.log(`共发现 ${files.length} 个会话日志文件。`)
    for (const file of files) {
      await repairSessionFile(file)
    }
    console.log('\n🎉 全量检查与修复完成！')
    return
  }

  const target = args[0]
  let targetPath = target

  if (!existsSync(targetPath)) {
    const files = await findSessionFiles(sessionsDir)
    const matched = files.find(f => f.includes(target))
    if (matched) {
      targetPath = matched
    } else {
      console.error(`❌ 未找到匹配的会话文件: ${target}`)
      process.exit(1)
    }
  }

  await repairSessionFile(targetPath)
}

main().catch(err => {
  console.error('执行失败:', err)
  process.exit(1)
})
