# Git 拓扑 / Worktree 状态图

> 快照:2026-06-26 合并 `docs` → `main` (`accc3da`) 之后
> 目的:梳理当前 5 个 worktree 各自承载的内容 + 所在分支是否合理

---

## 1. 完整分支树

```
                        ┌──── token-usage-c   @ c729670   (空, 0 提交, 80 文件)
                        │
                        ├──── token-usage-d   @ c729670   (空, 0 提交, 80 文件)
                        │
8334cad (Initial V0.1)  │
b861fa3 (rm PLAN.md)    │
25443cb (rm openspec)   │
c729670 (Tauri+Cargo) ──┤
                        │
                        ├─→ ui-update 5 提交
                        │   eb7c920  lane setup (stage-12/13 spec)
                        │   788f9bb  wave 0 (T-UI-001 / T-UI-002 / T5.5 usage API)
                        │   cfd21ac  stage-12 impl waves 1-4 (ui-redesign + token-usage-history)
                        │   b1a4b44  stage-12 + 13 status update
                        │   7f29c50  post-impl bug fixes (empty start + dark mode + categories)
                        │   ─→ HEAD @ 7f29c50
                        │
                        └─→ main / docs 共享 c729670 之后的线路
                            │
                            ├── main 旧线(直接到 main)
                            │   882c65e  Stage A (schema v3→v4 + PricingService + DB methods + tests)
                            │   3094e54  merge Stage A
                            │   accc3da  ★ current main HEAD — merge docs
                            │
                            └── docs 分支(private dev integration)
                                3598df2  add private dev content (.claude/, .omc/, .slim/, scripts/, PLAN, progress, handoff)
                                9fb864c / a8e4f22 / 6e7eb155f2f7 ...
                                5472999  .gitignore cleanup
                                562c7e8  DESIGN.md
                                98801d2  stage-12 review patches H/M/L ← 把 ui-update 的实现移植过来
                                0eaa9da  WIP cleanup V0.1.1
                                00ed9a2  merge Stage A
                                b9c7aef  Stage B: TokenUsageService (6 methods + 13 tests)
                                eadc5f9  A1 fix: per-dim cost split via TokenUsageCostBreakdown
                                aa888fd  merge token-usage-b
                                ─→ HEAD @ aa888fd
```

---

## 2. Worktree 矩阵

| # | Worktree 路径 | 分支 | HEAD | 提交数 / 文件数 | 实际内容 | 状态 |
|---|---|---|---|---|---|---|
| 1 | (主目录) | `main` | `accc3da` | 19 ↑ / 162+ | main = docs 全部内容(Stage A + B + Stage-12 + 私有工作流) | ✅ 真源 |
| 2 | (主目录, 备) | `docs` | `aa888fd` | 17 ↑ / 162+ | 合并前的内容,现在与 main 完全一致 | 🟡 与 main 重复 |
| 3 | `.claude/worktrees/ui-update` | `ui-update` | `7f29c50` | 5 ↑ / 162 | Stage 12 原始实现 + 修复(wave 0–4 + post-impl fixes) | 🟠 内容已在 main(经 docs 98801d2 移植) |
| 4 | `.claude/worktrees/token-usage-c` | `token-usage-c` | `c729670` | 0 ↑ / 80 | 空 WT,只看到 Stage 6 base,**没有 Stage A/B** | 🔴 base 过期,工作没起步 |
| 5 | `.claude/worktrees/token-usage-d` | `token-usage-d` | `c729670` | 0 ↑ / 80 | 同上 | 🔴 同上 |
| 6 | `.claude/worktrees/token-usage-e` | `token-usage-e` | `c729670` | 0 ↑ / 80 | 同上 | 🔴 同上 |

---

## 3. 内容归属分析(按"哪个分支该拥有"维度)

| 内容 | 当前所在 | 应归属分支 | 备注 |
|---|---|---|---|
| V0.1 credentials 主体 | main + docs + c729670 (所有) | `main` | 早期代码,所有人共享 |
| Stage A: schema v4 + PricingService + DB methods | main (3094e54 merge) | `main` | 已在 main |
| Stage B: TokenUsageService + A1 fix | main (经 docs 合并) | `main` | 已在 main |
| Stage-12 UI redesign 主实现 | ui-update (`cfd21ac`) | `ui-update` (源) / `main` (经 docs 98801d2 移植) | 两边都有,docs/main 是 review 后的版本 |
| Stage-12 review patches H/M/L | docs (`98801d2`) → main | `main` | 已在 main |
| Stage-12 post-impl bug fixes | ui-update (`7f29c50`) | ❓ **未合入** | `empty start + dark mode + real categories + theme toggle` 在 main 缺,需确认是否被 docs `98801d2` 覆盖 |
| Private 工作流 (.omc / .claude / .slim / PLAN / progress / handoff / DESIGN) | docs + main(刚合) | `main` | 已在 main,docs 历史保留 |
| `openspec/changes/ui-redesign/` | docs + main | `main` | 已在 main |
| Token-usage C/D/E 实际工作 | **不存在** | — | 三个分支空,从未起步 |

---

## 4. 问题与建议(请用户判断)

### 4.1 重复 / 冗余

- **`docs` 与 `main` 内容完全相同** — 保留 docs 作为历史锚点 OK,但日常使用认 `main`
- **`ui-update` 5 提交与 main 内容大量重叠** — Stage-12 主实现在 main(经 docs 移植),`7f29c50 post-impl fixes` 是否在 main 里需二次确认(看 `98801d2` 是不是包含)

### 4.2 base 过期

- **`token-usage-c/d/e` 停在 `c729670`** — 比 main 落后 19 提交,看不到 Stage A/B + Stage-12 代码。在这些 WT 里写新代码会与 main 严重脱节
- 若 Stage C/D/E 真要启动,**必须先 rebase 到 `accc3da` (或当前 main HEAD)**,否则依赖断链

### 4.3 提议的"正确分支"映射(草案,等用户拍板)

| 路径 | 现状 | 提议 |
|---|---|---|
| 主目录 | `main` | **保留** |
| `.claude/worktrees/ui-update` | `ui-update` @ `7f29c50` | **保留** + rebase 到 main(若继续 UI 后续工作);或 **删除**(若 7f29c50 内容已确认合入 main) |
| `.claude/worktrees/token-usage-c` | `token-usage-c` @ `c729670` | **删除**(0 提交,未起步)或 rebase 到 main 后重命名 `bd/token-usage-c/...` |
| `.claude/worktrees/token-usage-d` | `token-usage-d` @ `c729670` | 同上 |
| `.claude/worktrees/token-usage-e` | `token-usage-e` @ `c729670` | 同上 |
| `docs` (无 WT) | `docs` @ `aa888fd` | **保留** 作为历史锚,主仓日常用 main |

---

## 5. 待确认 / 阻塞项

1. **ui-update `7f29c50` 的 4 项修复 (empty start / dark mode / real categories / theme toggle) 是否在 main 里?**
   - 验证方法:对 main 跑这 4 个特性的 grep / test
2. **token-usage-c/d/e 还要用吗?**
   - 不用 → 删 WT + 删分支
   - 用 → 3 个分支合并成一个(避免无意义的多分支),或按 §4.4 重命名为 `bd/<task-id>/...`

---

*生成: 2026-06-26 · 数据源: `git log --all --graph` + `git worktree list` + 各 WT 文件统计*
