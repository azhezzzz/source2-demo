# `on_entity_property_changed` 分支改动整理

## Fork 来源

- 当前仓库：`https://github.com/azhezzzz/source2-demo`
- 上游来源：`https://github.com/Rupas1k/source2-demo`
- 本文整理的对比分支：`master...on_entity_property_changed`
- 本次 compare 的 `master` 基线提交：`de4ab4263984cc9326d29bfcb17a001db366a78b`
- 当前 `on_entity_property_changed` 分支头提交：`03160232d699cfcde9800e4cdfb2532ff997cf99`

这条分支是在上游项目 `Rupas1k/source2-demo` 的基础上维护的功能分支，用于给解析器补充“实体属性级别变更通知”能力，并保留后续同步上游时可继续 rebase / merge 的空间。

从提交历史看，这条分支中包含一次上游同步：

- `0400de7 Merge branch 'Rupas1k:master' into on_entity_property_changed`

说明该分支不是完全脱离上游单独演化，而是在持续吸收上游变更的基础上叠加本地功能。

## 这条分支的主要目标

为观察者系统增加 `on_entity_property_changed` 回调，使调用方可以按“单个属性”而不是“整个实体”接收变化通知。

触发语义如下：

- 实体创建时：对当前实体上已经填充的每个属性，各触发一次回调
- 实体更新时：对本次 packet 中实际发生变化的每个属性，各触发一次回调
- 实体删除时：不触发属性变更回调

支持的宏写法：

```rust
#[on_entity_property_changed("CDOTA_PlayerResource")]
```

```rust
#[on_entity_property_changed(
    class_pattern = "CDOTA_Unit_Hero_.*",
    property_pattern = "m_iHealth|m_iMaxHealth"
)]
```

## 提交概览

`master...on_entity_property_changed` 范围内的提交：

1. `c273b4e` Use checked slice reader refill
2. `0400de7` Merge branch 'Rupas1k:master' into on_entity_property_changed
3. `11c6684` Remove unnecessary comments
4. `d6d5482` Remove rustdoc html_root_url
5. `a00a78c` add Entity::field_paths and document entity snapshot tradeoffs
6. `174704a` Add upstream sync helper script
7. `e3da6f7` Add on_entity_property_changed observer support
8. `16c7fa3` Update hashbrown version

其中真正围绕 `on_entity_property_changed` 的核心功能主要集中在：

- `e3da6f7`
- `a00a78c`

其余提交多为上游同步、依赖调整、辅助脚本和小规模清理。

## 改动归类

### 1. 观察者接口层新增了属性级事件

涉及文件：

- `source2-demo/src/parser/observer.rs`
- `source2-demo/src/lib.rs`
- `source2-demo-macros/src/lib.rs`

主要改动：

- 在 `Observer` trait 中新增：

```rust
fn on_entity_property_changed(
    &mut self,
    ctx: &Context,
    entity: &Entity,
    field_path: &FieldPath,
) -> ObserverResult
```

- 在 `Interests` 中新增 `TRACK_ENTITY_PROPERTY`
- 在宏系统中新增 `#[on_entity_property_changed(...)]`
- 为宏生成代码补充内部过滤器导出：
  - `source2_demo::__private::PatternKind`
  - `source2_demo::__private::EntityPropertyPatternFilter`

这意味着调用方可以像注册 `on_entity` 一样，直接在 observer impl 上声明属性级回调，并通过 `Interests` 打开对应追踪能力。

### 2. 宏层支持类名 / 属性名过滤

涉及文件：

- `source2-demo-macros/src/lib.rs`
- `source2-demo/src/parser/observer.rs`

新增能力：

- `#[on_entity_property_changed("ExactClassName")]`
  - 单字符串形式只做“类名精确匹配”
- `#[on_entity_property_changed(class_pattern = "...", property_pattern = "...")]`
  - 命名参数形式支持 regex
- 同时兼容 camelCase：
  - `classPattern`
  - `propertyPattern`

运行时实现上：

- 类名匹配结果缓存为 `class -> bool`
- 属性匹配结果缓存为 `class -> (FieldPath -> bool)`
- regex 由 `regex` crate 编译并复用

这样做的目的，是避免在每次属性通知时都把类名和属性名重新做完整字符串匹配。

### 3. `FieldPath` 从内部类型变成了可公开使用的 API

涉及文件：

- `source2-demo/src/entity/field/path.rs`
- `source2-demo/src/entity/field/mod.rs`
- `source2-demo/src/entity/mod.rs`
- `source2-demo/src/lib.rs`

主要改动：

- `FieldPath` 由 `pub(crate)` 改为 `pub`
- 为 `FieldPath` 增加 `Eq + Hash + PartialEq`
- 在 `prelude` 中重新导出 `FieldPath`

这是属性变更回调可以对外工作的前提，因为回调参数本身就需要把“发生变化的字段路径”暴露给调用方。

### 4. 实体和类对象补充了围绕 `FieldPath` 的访问能力

涉及文件：

- `source2-demo/src/entity/class.rs`
- `source2-demo/src/entity/mod.rs`

新增 API：

- `Class::field_name_for_path(&FieldPath) -> String`
- `Entity::get_property_by_field_path(&FieldPath) -> Result<&FieldValue, EntityError>`
- `Entity::field_paths() -> Vec<FieldPath>`

作用分别是：

- 把解码后的 `FieldPath` 转回点分隔属性名
- 在已知 `FieldPath` 的情况下直接取属性值
- 枚举当前实体状态里已经存在的全部字段路径

其中 `Entity::field_paths()` 是实体创建时“逐属性触发回调”的关键辅助接口。

### 5. 底层字段解码器开始保留“本次变更了哪些字段”

涉及文件：

- `source2-demo/src/reader/field/mod.rs`

主要改动：

- `FieldReader::read_fields(...)` 不再只是写入状态，改为返回本次解码出的字段数量 `usize`
- 新增 `FieldReader::field_paths(count) -> &[FieldPath]`

这一步是整个功能的核心基础。没有这层改动，解析器只能知道“实体状态被更新了”，但不知道“具体是哪些属性在这次 packet 中发生了变化”。

### 6. 实体创建 / 更新路径增加逐属性派发

涉及文件：

- `source2-demo/src/parser/demo/svc.rs`

行为变化：

- `entity_created(...)`
  - 先照常完成实体构建
  - 先触发原有的 `on_entity(Created, ...)`
  - 然后枚举当前实体中所有已填充字段
  - 对每个字段依次触发 `on_entity_property_changed(...)`

- `entity_updated(...)`
  - 先用 `FieldReader` 解码本次更新
  - 保留本次变更的字段路径列表
  - 先触发原有的 `on_entity(Updated, ...)`
  - 然后仅对本次发生变化的字段触发 `on_entity_property_changed(...)`

- `entity_deleted(...)`
  - 没有额外加入属性级回调

这里保留了一个很重要的语义：属性回调看到的始终是“更新后的实体状态”，而不是更新前状态。

### 7. 依赖和辅助维护能力有同步调整

涉及文件：

- `source2-demo/Cargo.toml`
- `scripts/sync-upstream.sh`

改动内容：

- `hashbrown` 从 `0.16` 升级到 `0.17`
- 新增 `regex = "1.12"` 依赖，用于属性过滤
- 新增 `scripts/sync-upstream.sh`
  - 用于从上游抓取更新、rebase 当前功能分支，并执行 `cargo check` / `cargo test --lib`

这个脚本本身也说明了该分支的维护方式：本地功能分支需要周期性与 upstream 对齐，而不是长期脱离上游。

### 8. 其他非功能性调整

涉及文件：

- `source2-demo/src/reader/slice.rs`
- `source2-demo/src/parser/mod.rs`
- `source2-demo/src/reader/msg.rs`
- `source2-demo/src/reader/seekable.rs`
- `source2-demo/src/entity/field/decoder.rs`
- `source2-demo/src/entity/field/type.rs`
- `source2-demo/src/lib.rs`

主要内容：

- `SliceReader::refill()` 改为统一走 checked `refill_lookahead()`
- 删除若干注释
- 删除 `#![doc(html_root_url = "...")]`

这些改动和 `on_entity_property_changed` 不是同一层面的功能，但属于该分支相对于 `master` 的实际差异，需要在同步上游时一起关注。

## 文件清单

`master...on_entity_property_changed` 范围内涉及：

- `ON_ENTITY_PROPERTY_CHANGED.md`
- `scripts/sync-upstream.sh`
- `source2-demo-macros/src/lib.rs`
- `source2-demo/Cargo.toml`
- `source2-demo/src/entity/class.rs`
- `source2-demo/src/entity/field/decoder.rs`
- `source2-demo/src/entity/field/mod.rs`
- `source2-demo/src/entity/field/path.rs`
- `source2-demo/src/entity/field/type.rs`
- `source2-demo/src/entity/mod.rs`
- `source2-demo/src/lib.rs`
- `source2-demo/src/parser/demo/svc.rs`
- `source2-demo/src/parser/mod.rs`
- `source2-demo/src/parser/observer.rs`
- `source2-demo/src/reader/field/mod.rs`
- `source2-demo/src/reader/msg.rs`
- `source2-demo/src/reader/seekable.rs`
- `source2-demo/src/reader/slice.rs`

## 同步上游时需要优先关注的冲突点

高风险文件：

- `source2-demo/src/parser/demo/svc.rs`
  - 实体创建 / 更新流程最容易被上游改动影响
- `source2-demo/src/reader/field/mod.rs`
  - 如果上游改了字段解码流程，必须保住“返回本次变更字段路径”的能力
- `source2-demo-macros/src/lib.rs`
  - observer 宏入口大、逻辑集中，和上游演化最容易冲突

中风险文件：

- `source2-demo/src/parser/observer.rs`
- `source2-demo/src/entity/mod.rs`
- `source2-demo/src/entity/class.rs`
- `source2-demo/src/entity/field/path.rs`

## 需要保持的行为约束

- `on_entity_property_changed(...)` 必须看到更新后的实体状态
- 实体创建时应按“当前已有属性”逐个通知，而不是按 serializer schema 全量通知
- 实体删除时不触发属性变更回调
- `#[on_entity_property_changed("ClassName")]` 必须保持精确匹配语义
- `class_pattern` / `property_pattern` 必须保持 regex 语义
- 过滤结果需要缓存，避免每次回调重复做属性名匹配

## 一句话总结

这条分支本质上是在 `source2-demo` 现有实体观察机制之上，补了一层“按属性粒度派发”的能力；为此不仅新增了 observer 宏和 trait 方法，还把 `FieldPath`、字段路径解码结果、实体属性访问接口、运行时过滤缓存和上游同步脚本一起补齐了。
