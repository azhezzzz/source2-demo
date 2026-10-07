# `on_entity_property_changed` 同步历史

本文档只保留有追溯价值的上游同步记录，不描述当前操作流程或当前功能语义。

- 当前功能说明：[`ON_ENTITY_PROPERTY_CHANGED.md`](ON_ENTITY_PROPERTY_CHANGED.md)
- 当前更新流程：[`UPSTREAM_UPDATE_WORKFLOW.md`](UPSTREAM_UPDATE_WORKFLOW.md)
- 当前比较基线：[`UPSTREAM_BASELINE.md`](UPSTREAM_BASELINE.md)

下列记录反映合并发生时的状态，不得作为当前操作规范。更早的实现细节和完整 diff 可通过 Git 历史查询。

## 2026-10-07：更新到 0.5.9

- 合并前 `origin/master`：`c9f91d90c7a49fd3a19d7c3bd5963b6d0f634b9f`
- 合并目标 `origin/master`：`b1b1d504a29267e15b4d925126ddfa1f1e6b746d`
- merge commit：`2b0e15c1117ca0226cf1917233eca56e6b249498`
- 上游新增 11 个提交，版本更新到 `0.5.9`
- 主要变化：protobuf 更新、Deadlock combat log、CS2 QAngle 修复、`fixed8` 编解码修复
- 冲突文件：`source2-demo-macros/src/type_utils.rs`、`source2-demo/src/lib.rs`
- `fixed8` 更新解决了 `9032881092.dem` 的异常 entity index panic
- 上游新增 `CITADEL_COMBAT_LOG_ENTRIES = 1 << 18` 后，本分支将
  `TRACK_ENTITY_PROPERTY` 调整为 `1 << 19`，对应 commit `1d34dd4`

## 2026-07-13：更新到 0.5.8

- 合并前 `origin/master`：`71d1943908225687a906bd9b7cea3d3569995ade`
- 合并目标 `origin/master`：`c9f91d90c7a49fd3a19d7c3bd5963b6d0f634b9f`
- merge commit：`48824a0caea7e358257ef67a4141dd5a3f2a54ff`
- 主要变化：game event definitions 导出、demo writer 重构、版本更新到 `0.5.8`
- 没有内容冲突，属性变化核心链路未被上游触碰

## 2026-07-06：更新到 0.5.7

- 合并前 `origin/master`：`9909e369e6f308291ea15ef9e9dfd1206f86956c`
- 合并目标 `origin/master`：`71d1943908225687a906bd9b7cea3d3569995ade`
- merge commit：`7f905c9`
- 主要变化：新增 `Entity::fields()`、`EntityField`、baseline inspection、
  `FieldValue::type_name()` 和 `Context::previous_tick()`
- 本分支的全量实体检查场景改用上游 `Entity::fields()`，收缩了本地实现

## 2026-06-30：更新到 0.5.6

- 合并目标 `origin/master`：`9909e369e6f308291ea15ef9e9dfd1206f86956c`
- merge commit：`e4d0dc8797379ef45881a0aba3dd9e2eb875ed25`
- 主要变化：protobuf 定义和生成物更新
- 没有内容冲突，属性变化核心链路未受影响

## 2026-06-29：合并 checked entity command read

- 合并目标 `origin/master`：`12a5bb1547220e20f01995fc1cbeffb89f8c501c`
- merge commit：`c74d1fe234ff4b6af344d8e1ca2f775cefca6900`
- 上游将 entity command 读取从 unchecked 改为 checked read
- 没有内容冲突或属性变化语义变化

## 2026-06-22：更新到 0.5.5

- 合并目标 `origin/master`：`4be471b1cc7bb4731e403367ddcacf94dd5a38df`
- merge commit：`00d4f60befc7fb914a79b8d8a5cca26adb79674f`
- 主要变化：send node 前缀进入字段名、writer 修复、版本更新到 `0.5.5`
- 本分支保留上游字段名语义，下游负责兼容新旧字段路径

## 2026-06-16：更新到 0.5.4

- 合并目标 `origin/master`：`5dd12b587c0653a96f1c844103fd13d9aea34ace`
- merge commit：`2a2c903b1aa60ca9b174358c3c5d8978c7d29a56`
- 主要变化：demo message rewriter 和版本更新
- 没有内容冲突

## 2026-06-08：更新到 0.5.2 并收缩分支差异

- 合并目标 `origin/master`：`1eb90381d063eb0feef10110322d303b6e02d01c`
- merge commit：`1b14ca5`
- 兼容修复：`6f0d66e`
- 最小化收缩：`56fa66e`
- 主要冲突是 `FieldPath` 可见性；继续保持公开以支持下游批量属性变化回调
- `field_name_for_path(...)` 在公共 API 边界适配上游 `Rc<str>` 返回值

## 更早记录

2026-05 的早期实现曾包含单字段回调、regex 过滤和不同的 entity 创建触发语义，之后均已
删除。这些设计不得恢复；如需考古，使用 Git 历史查看旧版 `ON_ENTITY_PROPERTY_CHANGED.md`。
