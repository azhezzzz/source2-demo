# 上游更新流程

本文档是 `on_entity_property_changed` 分支同步上游的唯一当前操作规范。
当前比较基线见 `UPSTREAM_BASELINE.md`，当前功能说明见 `ON_ENTITY_PROPERTY_CHANGED.md`，
精简同步历史见 `ON_ENTITY_PROPERTY_CHANGED_HISTORY.md`。如有冲突，以本文档为准。

## 给新会话的执行入口

新会话收到“更新上游”“更新合并上游”“同步 master”或含义相同的请求时，必须先完整
阅读本文档，再执行任何 fetch、merge、checkout、commit 或 push。无需依赖既往对话，按
本文档即可完成一次更新。

默认把请求理解为：分析并将 GitHub fork 的 `origin/master` 合入
`on_entity_property_changed`。不要把它理解为直接从官方仓库拉取并合并，也不要默认执行
commit、push 或主仓库 gitlink 提交。

如果用户只要求“对比”或“分析”，流程在第 4 步报告分析结果后结束，不执行 merge。
如果用户要求“更新合并”，仍必须先报告分析结果，并在得到明确确认后才能 merge。

## 强制约束

- 操作前先确认当前目录、分支、工作区状态和 remote
- 保留用户已有的未提交修改；工作区不干净且与合并范围重叠时停止并说明
- 用户未确认前不得 merge 或 rebase
- 默认使用 merge，不把定制分支 rebase 到上游
- 不直接合并官方 `Rupas1k/source2-demo:master`，只合并同步后的 `origin/master`
- 优先复用上游实现，只保留上游尚未覆盖且主项目仍真实依赖的最小补丁
- 不修改 `references/parser_rust_ro` 以外的无关文件
- 不手动编辑生成物，不把 `.dem`、`target/`、临时输出或机器路径加入提交
- commit、push 和主仓库 gitlink 提交都需要用户分别明确授权
- native、Node / wasm 和 benchmark 验证必须串行执行，不得并行

## 仓库与分支关系

- 官方上游：`Rupas1k/source2-demo:master`
- GitHub fork：`azhezzzz/source2-demo`
- fork 上的同步分支：`master`
- 本项目使用的定制分支：`on_entity_property_changed`
- 本地依赖仓库：`references/parser_rust_ro`

用户先在 GitHub 上把 fork 的 `master` 与官方上游同步。本地不直接把官方仓库的
`master` 合入定制分支，而是以同步后的 `origin/master` 为唯一待合并基线。

## 标准流程

### 0. 执行前检查

从主仓库根目录确认：

```bash
git status --short
git ls-files --stage references/parser_rust_ro
git -C references/parser_rust_ro status --short --branch
git -C references/parser_rust_ro remote -v
git -C references/parser_rust_ro branch --show-current
```

必须确认：

- `references/parser_rust_ro` 在主仓库中是 mode `160000` 的 gitlink
- 内部当前分支是 `on_entity_property_changed`
- `origin` 指向 `azhezzzz/source2-demo`
- 内部工作区没有未知或未处理的修改
- 主仓库的无关修改和未跟踪文件不会被本次操作带入

任一条件不满足时，不要自行清理、reset 或覆盖；先报告现状。

### 1. 用户先同步 GitHub fork

由用户在 GitHub 上完成：

```text
Rupas1k/source2-demo:master -> azhezzzz/source2-demo:master
```

用户确认同步完成后，才开始本地流程。

### 2. 刷新远端引用

在 `references/parser_rust_ro` 中执行：

```bash
git fetch origin
```

fetch 只用于刷新引用，不应立即 merge 或 rebase。

### 3. 分析上游变化

从 `UPSTREAM_BASELINE.md` 取得上次已合并的 `origin/master` commit，并比较：

```bash
git log --oneline <baseline>..origin/master
git diff --stat <baseline>..origin/master
git diff --name-status <baseline>..origin/master
```

同时确认分叉状态和预演冲突：

```bash
git rev-list --left-right --count HEAD...origin/master
git merge-base HEAD origin/master
git merge-tree <merge-base> HEAD origin/master
```

分析结果至少应包含：

- 上次基线 commit 与当前 `origin/master` commit
- 新增提交及主要功能变化
- 是否触碰当前分支热点文件
- 是否已有上游能力可以替代或收缩本地实现
- 预计冲突、行为变化和验证风险
- 是否建议合并及理由

热点能力包括：

- `TRACK_ENTITY_PROPERTY`
- `#[on_entity_properties_changed]`
- `FieldPath` 的公开边界
- `Entity::get_property_by_field_path(...)`
- `Class::field_name_for_path(...)`
- `Class::field_type_for_path(...)`
- `FieldReader::field_paths(...)`

如果上游修改了 `Interests`，必须人工检查所有 flag 是否使用唯一 bit，尤其要确认
本分支私有的 `TRACK_ENTITY_PROPERTY` 未与上游 flag 重复。发生冲突时优先移动本分支
私有 flag，不修改上游已有 flag。

还必须检查上游是否触碰这些高风险文件：

- `source2-demo/src/parser/demo/svc.rs`
- `source2-demo/src/parser/observer.rs`
- `source2-demo/src/stream/reader/field.rs`
- `source2-demo-macros/src/observer_impl.rs`
- `source2-demo-macros/src/type_utils.rs`
- `source2-demo/src/entity/field/path.rs`
- `source2-demo/src/entity/mod.rs`
- `source2-demo/src/entity/class.rs`

对每个重叠点都要判断：上游是否已经提供等价能力；如果提供，应优先删除或收缩本地
实现，而不是机械保留双方代码。

### 4. 等待用户确认

分析完成后先报告结果。在用户明确确认前，不执行：

```bash
git merge origin/master
```

### 5. 合并并解决冲突

用户确认后执行：

```bash
git merge origin/master
```

冲突处理遵循以下顺序：

1. 优先采用上游现有实现
2. 上游已有等价能力时删除本地重复实现
3. 上游只有相似能力时让本地实现靠拢上游接口
4. 仅保留主项目仍真实依赖、且上游尚未覆盖的最小增量
5. 不直接修改上游已有 flag、协议或公共语义来迁就本分支

解决后必须检查：

```bash
git status --short
git diff --check
rg -n '^(<<<<<<<|=======|>>>>>>>)' .
```

并确认以下行为没有丢失：

- entity 创建时不触发属性变化回调
- entity 更新时一次回调返回本 packet 的全部变化 `FieldPath`
- entity 删除时不触发属性变化回调
- 回调读取到更新后的 entity 状态
- `TRACK_ENTITY_PROPERTY` 与所有上游 interest 使用不同 bit

### 6. 验证

至少执行：

```bash
cargo fmt --all -- --check
cargo test
```

如果合并涉及 feature gate、游戏专用功能或 `Interests`，按需执行：

```bash
cargo test --all-features
```

然后在主仓库执行：

```bash
cargo check --manifest-path core_rust/Cargo.toml
```

涉及解析行为时，还应使用指定录像做 native 验证。native 与 Node / wasm 验证必须串行执行。

验证失败时不要把失败状态描述为完成，也不要为通过测试而删除本分支必要能力。应先定位
失败来自上游行为变化、冲突处理还是下游 API 不兼容，再报告结果。

### 7. 更新记录

更新 `ON_ENTITY_PROPERTY_CHANGED.md` 的“最新上游同步摘要”和验证状态，记录：

- 日期
- 合并前基线 `origin/master` commit
- 本次目标 `origin/master` commit
- merge commit
- 纳入的上游提交摘要
- 冲突及处理方式
- 本地能力保留或收缩情况
- 验证命令和结果

如优化方向发生变化，同时更新 `OPTIMIZATION_DIRECTIONS.md`。

同时更新 `UPSTREAM_BASELINE.md`，只保留最新一次已合并的 `origin/master` commit、
版本和 merge commit。该文件是下一次差异分析的唯一基线来源，不追加历史记录。

### 8. 提交、推送与主仓库 gitlink

`references/parser_rust_ro` 是独立 Git 仓库。只有用户明确要求提交时，才在该目录内
提交；只有用户明确要求推送时，才推送 `on_entity_property_changed`。

依赖仓库提交后，主仓库会显示：

```text
M references/parser_rust_ro
```

这是 gitlink 更新。只有用户明确要求提交主仓库时，才在主仓库提交该 gitlink。
不要把 `.dem`、临时输出或其他无关文件带入提交。

提交前再次检查实际范围：

```bash
git -C references/parser_rust_ro status --short
git -C references/parser_rust_ro diff --check
git status --short
```

提交依赖仓库、推送依赖分支和提交主仓库 gitlink 是三个独立动作，不能因为用户授权其中
一个就推定另外两个也已获授权。

## 完成标准

一次上游更新只有同时满足以下条件才算完成：

- 使用的是用户已在 GitHub 同步后的 `origin/master`
- 合并前已完成差异和风险分析，并得到用户确认
- 冲突已解决且没有遗留 conflict marker
- 本分支必要能力仍存在，能由上游替代的重复实现已尽量收缩
- `Interests` 的所有 bit 唯一
- 约定的格式、测试、主仓库编译和必要解析验证已完成或明确标记为待用户验证
- `ON_ENTITY_PROPERTY_CHANGED.md` 已记录本次基线、目标、merge、冲突和验证结果
- `UPSTREAM_BASELINE.md` 已更新为本次合并使用的 `origin/master`
- 未提交或推送任何未经用户授权的内容

## 当前脚本状态

主仓库的 `scripts/update_parser_rust_ro.sh` 暂不可作为本流程入口，原因包括：

- 依赖当前仓库缺失的 `.gitmodules`
- 固定更新 `on_entity_property_changed`，没有执行基线差异分析
- 会自动 checkout、add 和 commit
- 不符合“先分析、用户确认后合并、明确要求后提交”的当前规则

在脚本按本文档重写前，应使用上述 Git 命令手动执行流程。
