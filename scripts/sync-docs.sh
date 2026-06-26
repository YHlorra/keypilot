#!/bin/bash
# sync-docs.sh — 代码与文档分支同步工具
#
# 用法:
#   ./scripts/sync-docs.sh check      检查两分支是否同步，退出码反映状态
#   ./scripts/sync-docs.sh merge      把 docs 合并到 main（或反之，见参数）
#   ./scripts/sync-docs.sh push-docs  推送到私有 remote
#   ./scripts/sync-docs.sh push-main  推送到公开 remote
#
# 前置条件:
#   git remote add origin-private <url>   # 配置私有 remote（可选）
#
# 分支约定:
#   main  — 代码
#   docs  — 文档（可包含代码变更的文档部分）

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(git rev-parse --show-toplevel)"

# ── 颜色 ──────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

info()  { echo -e "${GREEN}[sync-docs]${NC} $*"; }
warn()  { echo -e "${YELLOW}[sync-docs]${NC} $*"; }
error() { echo -e "${RED}[sync-docs]${NC} $*"; }

# ── 工具函数 ──────────────────────────────────────────
count_commits_behind() {
  # docs 比 main 落后多少个 commit
  git rev-list --count "$1".."$2" 2>/dev/null || echo "?"
}

branch_exists() {
  git show-ref --verify --quiet "refs/heads/$1" 2>/dev/null
}

# ── check ─────────────────────────────────────────────
cmd_check() {
  local main_head docs_head
  main_head=$(git rev-parse main 2>/dev/null) || { error "main 分支不存在"; exit 1; }

  if ! branch_exists docs; then
    warn "docs 分支不存在，首次使用请先: git checkout -b docs"
    exit 2
  fi
  docs_head=$(git rev-parse docs)

  if [ "$main_head" = "$docs_head" ]; then
    info "main 和 docs 同步（HEAD: ${main_head:0:8}）"
    exit 0
  fi

  local behind_ahead
  behind_ahead=$(git rev-list --left-right --count main...docs 2>/dev/null || echo "? ?")
  local behind ahead
  behind=$(echo "$behind_ahead" | cut -f1)
  ahead=$(echo "$behind_ahead" | cut -f2)

  if [ "$behind" != "?" ]; then
    warn "两分支已分叉: main 落后 docs $behind 个 commit，docs 落后 main $ahead 个 commit"
  else
    warn "两分支已分叉（无法计算精确差异）"
  fi
  echo "  main HEAD: ${main_head:0:8}"
  echo "  docs HEAD: ${docs_head:0:8}"
  echo ""
  echo "  同步建议: ./scripts/sync-docs.sh merge"
  exit 3
}

# ── merge ─────────────────────────────────────────────
cmd_merge() {
  local source="${1:-main}"  # 把 source 的变更合并进 target
  local target
  if [ "$source" = "main" ]; then
    target="docs"
  else
    target="main"
  fi

  if ! branch_exists "$source"; then error "$source 分支不存在"; exit 1; fi
  if ! branch_exists "$target"; then error "$target 分支不存在"; exit 1; fi

  local source_head target_head
  source_head=$(git rev-parse "$source")
  target_head=$(git rev-parse "$target")

  if [ "$source_head" = "$target_head" ]; then
    info "$source 和 $target 已同步，无需合并"
    return
  fi

  info "将 $source 合并进 $target ..."
  git checkout "$target"
  git merge "$source" --no-edit --no-ff || {
    error "合并冲突，请手动解决后重新运行此命令"
    exit 1
  }
  info "合并完成"
  cmd_check
}

# ── push ──────────────────────────────────────────────
cmd_push() {
  local target="${1:-main}"
  local remote="${2:-origin}"

  if ! branch_exists "$target"; then error "$target 分支不存在"; exit 1; fi

  # 检查 remote 是否存在
  if ! git remote get-url "$remote" >/dev/null 2>&1; then
    error "remote '$remote' 未配置。先执行: git remote add $remote <url>"
    exit 1
  fi

  local remote_url
  remote_url=$(git remote get-url --push "$remote" 2>/dev/null || git remote get-url "$remote")

  # 对公开 remote 额外警告
  case "$remote_url" in
    *github.com*|*gitlab.com*|*bitbucket.org*)
      case "$remote_url" in
        *-private*|*internal*|*private*) ;;
        *)
          if [ "$target" = "docs" ]; then
            error "试图将 docs 推到可能公开的 remote '$remote' ($remote_url)"
            error "使用 --force-public 绕过（不推荐）或配置私有 remote"
            exit 1
          fi
          ;;
      esac
      ;;
  esac

  info "推送 $target → $remote ..."
  git push "$remote" "$target"
  info "推送完成"
}

# ── 主入口 ────────────────────────────────────────────
case "${1:-}" in
  check)    cmd_check ;;
  merge)    cmd_merge "${2:-main}" ;;
  push-docs)  cmd_push docs "origin-private" ;;
  push-main)  cmd_push main "origin" ;;
  *)
    echo "用法: $0 <command> [args]"
    echo ""
    echo "命令:"
    echo "  check             检查 main/docs 同步状态"
    echo "  merge [source]    合并 source 进另一分支 (默认 source=main)"
    echo "  push-docs         推送到私有 remote (origin-private)"
    echo "  push-main         推送到公开 remote (origin)"
    echo ""
    echo "示例工作流:"
    echo "  git add . && git commit -m 'feat: xxx'"
    echo "  ./scripts/sync-docs.sh merge main    # 同步文档分支"
    echo "  ./scripts/sync-docs.sh push-main      # 代码推公开"
    echo "  ./scripts/sync-docs.sh push-docs      # 文档推私有"
    exit 1
    ;;
esac
