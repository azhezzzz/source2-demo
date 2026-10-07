# `on_entity_properties_changed` 当前分支说明

本文档只描述 `on_entity_property_changed` 分支当前仍有效的功能和相对上游保留的最小差异。

- 上游更新流程：[`UPSTREAM_UPDATE_WORKFLOW.md`](UPSTREAM_UPDATE_WORKFLOW.md)
- 当前比较基线：[`UPSTREAM_BASELINE.md`](UPSTREAM_BASELINE.md)
- 精简同步历史：[`ON_ENTITY_PROPERTY_CHANGED_HISTORY.md`](ON_ENTITY_PROPERTY_CHANGED_HISTORY.md)
- 后续收缩方向：[`OPTIMIZATION_DIRECTIONS.md`](OPTIMIZATION_DIRECTIONS.md)

旧版单字段回调、regex 过滤、旧 compare 口径和旧同步步骤均不再有效。

## 当前目标

在尽量复用上游实现的前提下，为 `source2-demo` 补充 entity 更新时的批量属性变化通知，
供下游只处理本 packet 实际变化的字段。

当前回调为：

```rust
fn on_entity_properties_changed(
    &mut self,
    ctx: &Context,
    entity: &Entity,
    field_paths: &[FieldPath],
) -> ObserverResult
```

## 当前行为语义

- entity 创建时不触发属性变化回调
- entity 更新时一次回调返回本 packet 的全部变化 `FieldPath`
- entity 删除时不触发属性变化回调
- 回调读取到的是更新后的 entity 状态
- 不支持旧版单字段 `on_entity_property_changed`
- 不支持 `class_pattern`、`property_pattern` 或 regex 过滤

## 当前必须保留的最小能力

1. `Interests::TRACK_ENTITY_PROPERTY`
2. `#[on_entity_properties_changed]`
3. `FieldPath` 的对外可见性
4. `Entity::get_property_by_field_path(&FieldPath)`
5. `Class::field_name_for_path(&FieldPath) -> String`
6. `Class::field_type_for_path(&FieldPath) -> String`
7. 内部 `FieldReader::field_paths(count)`

这些能力目前仍被主项目用于 entity 增量输出。上游提供等价能力前不能直接删除。

## `Interests` bit

本分支私有 flag 当前使用：

```rust
TRACK_ENTITY_PROPERTY = 1 << 19
```

上游 `0.5.9` 新增的 `CITADEL_COMBAT_LOG_ENTRIES` 使用 `1 << 18`。本分支已避让上游
flag。后续同步上游如涉及 `Interests`，必须人工检查所有 bit 是否唯一；发生冲突时优先
调整本分支私有 flag，不修改上游已有 flag。

## 相对上游保留的源码差异

当前功能涉及以下文件：

- `source2-demo-macros/src/lib.rs`
- `source2-demo-macros/src/observer_impl.rs`
- `source2-demo-macros/src/type_utils.rs`
- `source2-demo/src/entity/class.rs`
- `source2-demo/src/entity/field/mod.rs`
- `source2-demo/src/entity/field/path.rs`
- `source2-demo/src/entity/mod.rs`
- `source2-demo/src/parser/demo/svc.rs`
- `source2-demo/src/parser/observer.rs`
- `source2-demo/src/stream/reader/field.rs`

每次同步上游后应重新通过 `git diff origin/master...HEAD` 核对这份清单，不应机械沿用。

## 最小化入侵原则

- 能使用上游能力时优先使用上游能力
- 上游已有等价实现时删除本地重复实现
- 上游只有相似能力时让本地实现靠拢上游接口
- 只保留主项目真实依赖且上游尚未覆盖的能力
- 不为使用方便扩大上游 `prelude` 或公共 API
- 能修改下游完成的适配，不继续扩大 parser fork
- 不修改上游已有 flag、协议或公共语义来迁就本分支

当前优先收缩方向见 `OPTIMIZATION_DIRECTIONS.md`。

## 最新上游同步摘要

- 日期：`2026-10-07`
- 合并前基线：`c9f91d90c7a49fd3a19d7c3bd5963b6d0f634b9f`
- 已合并 `origin/master`：`b1b1d504a29267e15b4d925126ddfa1f1e6b746d`
- merge commit：`2b0e15c1117ca0226cf1917233eca56e6b249498`
- 文档更新 commit：`5795280`
- interest bit 修复 commit：`1d34dd4`
- 上游版本：`0.5.9`

本次上游更新包含 enum 和 handle 的 `fixed8` 支持，已解决主项目解析
`9032881092.dem` 时因 bitstream 错位产生异常 entity index 的 panic。

## 当前验证状态

- 上游 `cargo fmt --all -- --check`：通过
- 上游 `cargo test`：通过，单元测试 `31 passed`
- 上游 doc tests：`61 passed / 12 ignored`
- 主项目 `cargo check --manifest-path core_rust/Cargo.toml`：通过
- `9032881092.dem` native 解析：通过，不再 panic

## 一句话总结

本分支只在上游 entity observer 之上保留“批量返回本次更新字段路径”的最小增量；上游
解析、字段解码和其他游戏功能均应直接复用。
