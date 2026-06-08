# 优化方向整理

本文档只记录当前这条 `on_entity_property_changed` 分支后续应优先推进的优化方向，便于和 `ON_ENTITY_PROPERTY_CHANGED.md` 中的功能说明分离维护。

## 2026-06-08 基线更新

在 `2026-06-08` 合并最新 `origin/master` 后，已确认：

- 上游这次更新没有改变当前分支的属性变更语义
- 当前真正受影响的只有两处：
  - `FieldPath` 的可见性：上游希望收回成 `pub(crate)`，当前分支仍需继续公开
  - `field_name_for_path(...)` 的返回值边界：上游内部返回类型变为 `Rc<str>`，当前分支继续对外返回 `String`

同时也重新确认了主仓库 `/Volumes/SamsungExtra/dota_dem_parser_rust` 的真实依赖：

- 现在还不能直接退掉：
  - `TRACK_ENTITY_PROPERTY`
  - `#[on_entity_properties_changed]`
  - `FieldPath` 的对外可见性
  - `Entity::get_property_by_field_path(&FieldPath)`
- 现在还不适合立刻删，但应作为下一轮优先收缩目标：
  - `Entity::field_paths()`
  - `Class::field_name_for_path(&FieldPath)`

原因不是 parser fork 自己需要这些附加 API，而是主仓库当前仍然保留：

- `onStart.Extra.LastTickEntities` 的全量实体快照链路
- `changedFields` 仍按字段名导出的协议

因此后续“最小化改动”的重点，仍应优先放在主仓库侧收缩依赖，而不是先继续压缩 parser fork 的底层实现。

## 当前原则

- 优先通过“新增”实现能力，尽量避免直接修改上游现有源码
- 纯新增文件可以保留，例如文档、脚本、辅助说明
- 只有在当前主流程确实必须时，才修改 `source2-demo` 现有源码
- 如果必须修改现有源码，优先选择改动面最小、侵入性最低的实现

## 当前状态

相对 `origin/master`，当前分支已经删除：

- 单字段 `on_entity_property_changed`
- `class_pattern` / `property_pattern`
- `regex` 依赖

当前保留的核心能力是：

- `TRACK_ENTITY_PROPERTY`
- `#[on_entity_properties_changed]`
- `Entity::get_property_by_field_path(&FieldPath)`
- `Entity::field_paths()`
- `FieldReader::field_paths(count)`
- `Class::field_name_for_path(&FieldPath)`
- `FieldPath` 的对外可见性

其中对外公开 API 目前是：

- `Interests::TRACK_ENTITY_PROPERTY`
- `#[on_entity_properties_changed]`
- `FieldPath`
- `Entity::get_property_by_field_path(&FieldPath)`
- `Entity::field_paths()`
- `Class::field_name_for_path(&FieldPath)`

其中内部辅助接口目前是：

- `FieldReader::field_paths(count)`

## 优先优化方向

### 1. 优先检查 `LastTickEntities`

当前主仓库仍然通过 `onStart` 的全量实体快照使用 `LastTickEntities`。

应优先检查：

- 主仓库是否还真正需要 `LastTickEntities`
- Node 侧是否还真正消费 `LastTickEntities`

如果两边都不再需要，应优先删除这条链路。

### 2. 如果可以删除 `LastTickEntities`，继续评估移除 `Entity::field_paths()`

当前 `Entity::field_paths()` 主要被主仓库的全量实体快照路径使用。

如果 `LastTickEntities` 被删除，应继续评估：

- 主仓库是否还能完全去掉对 `Entity::field_paths()` 的依赖
- 如果可以，则这项相对 `origin/master` 的新增 API 也可以继续回退

### 3. 暂时保留 `Class::field_name_for_path(...)`

当前主仓库对外导出的增量更新协议仍然是：

- `changedFields: { fieldName: value }`

在这个协议不变的前提下，仍然需要：

- `FieldPath -> 字段名`

因此当前不应优先删除 `Class::field_name_for_path(...)`。

### 4. 只有当导出协议变化时，再评估移除字段名映射

如果未来主仓库不再按字段名导出，或者导出协议改成直接传路径/索引，再继续评估：

- 是否还能移除 `field_name_for_path(...)`
- 是否还能进一步减少 `FieldPath` 相关公开能力

## 评估顺序

建议按下面顺序推进，而不是同时改多处：

1. 检查并确认 `LastTickEntities` 是否可删除
2. 如果可删除，收缩主仓库的全量实体快照路径
3. 再判断 `Entity::field_paths()` 是否还能保留
4. 最后才评估 `field_name_for_path(...)` 和更深层的导出协议调整

## 当前判断

按当前主仓库和 Node 侧实现来看：

- `LastTickEntities` 仍在被真实消费，不只是历史文档残留
- `Entity::field_paths()` 不是 Node 侧直接需要的，而是主仓库自己的全量实体快照链路还在用
- `field_name_for_path(...)` 主要是当前增量导出协议还在用

因此当前最值得优先验证的，不是继续改底层 observer，而是：

- 先从主仓库侧确认 `LastTickEntities` 是否还能继续保留
