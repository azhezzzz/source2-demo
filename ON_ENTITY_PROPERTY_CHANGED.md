# `on_entity_property_changed` 分支改动整理

## Fork 来源

- 当前仓库：`https://github.com/azhezzzz/source2-demo`
- 上游来源：`https://github.com/Rupas1k/source2-demo`
- fork 内部分支对比：`azhezzzz/source2-demo:master...azhezzzz/source2-demo:on_entity_property_changed`
- GitHub 分支页的上游对比口径：`Rupas1k/source2-demo:master...azhezzzz:on_entity_property_changed`
- 当前上游 `master` 提交：`43677da88b114b5ee8a0dd7fa3ae6208799ffdcb`
- 当前 fork `master` 提交：`de4ab4263984cc9326d29bfcb17a001db366a78b`
- 本地当前 `on_entity_property_changed` 分支头提交：`d03be8059534db9ef2aeb1d4f4ae2808c2486164`

这条分支是在上游项目 `Rupas1k/source2-demo` 的基础上维护的功能分支，用于给解析器补充“实体属性级别变更通知”能力，并保留后续同步上游时可继续 rebase / merge 的空间。

从提交历史看，这条分支中已经包含两次上游同步：

- `0400de7 Merge branch 'Rupas1k:master' into on_entity_property_changed`
- `d03be80 Merge upstream/master into on_entity_property_changed`

说明该分支不是完全脱离上游单独演化，而是在持续吸收上游变更的基础上叠加本地功能。

## 2026-05-18 最新状态

以下内容优先级高于本文后面保留的历史分析；后面的旧小节主要用于记录之前的判断过程。

### 当前同步结论

- 已在 `2026-05-18` 将 `upstream/master` 合并进当前分支
- 当前 compare 口径仍然是 `Rupas1k/source2-demo:master...azhezzzz:on_entity_property_changed`
- 当前状态：`ahead_by = 11`
- 当前状态：`behind_by = 0`
- 当前分支已经吸收 `upstream/master` 到 `43677da88b114b5ee8a0dd7fa3ae6208799ffdcb`

### 本次合并纳入的上游提交

1. `93950fb` Add get_row method for StringTable
2. `5009b21` fmt + protobufs
3. `02591a9` Change player_name type from string to bytes in build.rs (#12)
4. `736f515` Fix field path name getter for arrays
5. `43677da` Implement add_observer method

### 本次实际冲突

- 实际冲突文件只有一个：`source2-demo/src/parser/observer.rs`

冲突原因：

- 当前分支在这里增加了 `on_entity_property_changed` 相关的过滤器、`FieldPath` 事件和运行时缓存
- 上游在这里增加了 `Rc<RefCell<T>>` 的 `Observer` 实现，用于配合新的 `Parser::add_observer(...)`

处理结果：

- 两边改动都保留
- 当前分支继续保留：
  - `TRACK_ENTITY_PROPERTY`
  - `on_entity_property_changed(...)`
  - `PatternKind`
  - `EntityPropertyPatternFilter`
- 同时吸收上游新增能力：
  - `Parser::add_observer(...)`
  - `observers: Vec<Box<dyn Observer>>`
  - `Rc<RefCell<T>>` 的 `Observer` 转发实现

### merge 后的行为判断

对 `on_entity_property_changed` 需求本身的判断：

- 属性级回调语义没有变化
- 实体创建时仍然按当前已有属性逐个通知
- 实体更新时仍然只对本次变更字段通知
- 实体删除时仍然不触发属性级回调
- 回调看到的仍然是更新后的实体状态

本次合并后新增的上游能力：

- 可以通过 `Parser::add_observer(observer)` 注册带自定义初始状态的 observer
- `register_observer::<T>()` 现在内部复用 `add_observer(T::default())`

### 已完成验证

- `cargo check`
- `cargo test --lib`

两者均已通过。

## 对比口径说明

这个项目后续应优先按 GitHub 分支页的上游关系来理解，而不是只按你自己 fork 里的 `master` 来理解。

### 1. fork 内部 compare

对应链接：

- `https://github.com/azhezzzz/source2-demo/compare/master...on_entity_property_changed`

这个 compare 的 base 是你 fork 自己的 `master`，不是上游 `Rupas1k/source2-demo:master`。

它适合回答的问题是：

- 你的 fork 中，`on_entity_property_changed` 相对你 fork 的 `master` 改了什么

### 2. GitHub 分支页 / upstream parent compare

对应链接：

- `https://github.com/azhezzzz/source2-demo/tree/on_entity_property_changed`

这个分支页背后的对比关系，应理解为：

- `Rupas1k/source2-demo:master...azhezzzz:on_entity_property_changed`

截至 2026-05-06、且在本次合并 `upstream/master` 之前，本页对应的 GitHub compare 元数据为：

- 状态：`diverged`
- `ahead_by = 5`
- `behind_by = 5`
- 上游当前 `master` 头提交：`1ff430918006463520c2e80fa33a185bfe73dd4b`
- 共同祖先：`11c6684ae5a325f6ef97301ecfa59dd160495498`

这也是后续判断“当前分支落后上游哪些提交、同步上游会不会冲突”时应该使用的基线。

### 3. 下次让 Codex 对比时的固定说法

建议直接使用下面这句：

```text
按 GitHub 分支页 https://github.com/azhezzzz/source2-demo/tree/on_entity_property_changed 的上游关系，对比 Rupas1k/source2-demo:master 和 azhezzzz:on_entity_property_changed。
```

如果要我进一步分析冲突和影响，可以扩展成：

```text
按 GitHub 分支页 https://github.com/azhezzzz/source2-demo/tree/on_entity_property_changed 的上游关系，对比 Rupas1k/source2-demo:master 和 azhezzzz:on_entity_property_changed，列出上游新增提交、冲突文件、对 on_entity_property_changed 功能的影响，并给出 merge/rebase 建议。
```

也可以进一步简化成：

```text
对比上游最新 master。
```

在这个仓库里，如果你这样说，默认应理解为：

- 按 GitHub 分支页 `https://github.com/azhezzzz/source2-demo/tree/on_entity_property_changed` 的上游关系进行对比
- 实际比较口径是 `Rupas1k/source2-demo:master...azhezzzz:on_entity_property_changed`
- 自动列出：
  - 当前 `ahead / behind`
  - 上游新增提交
  - 冲突文件
  - 对 `on_entity_property_changed` 分支的影响
  - 推荐同步策略

### 4. 默认同步策略

这个分支后续应采用“尽量回归上游、减少本地长期维护”的策略。

具体原则：

- 如果上游已经提供了等价实现，优先删除本地自定义实现，改用上游方案
- 如果上游提供了相似但不完全相同的实现，优先把本地改动收敛到上游接口或上游设计上
- 只有在上游没有覆盖当前需求时，才继续保留本地补丁
- 做影响分析时，不只是判断“会不会冲突”，还要判断“能不能把本地改动回退掉，改成上游方法”

因此，后续做 compare 分析时，默认目标不是“保住所有本地实现”，而是：

- 保住 `on_entity_property_changed` 这个需求本身
- 尽可能减少自维护代码
- 尽可能把实现方式对齐到上游

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

## 基于 GitHub 分支页口径的分叉情况（本次合并前）

按 `Rupas1k/source2-demo:master...azhezzzz:on_entity_property_changed` 计算：

### 当前分支领先上游的 5 个提交

1. `e3da6f7` Add on_entity_property_changed observer support
2. `174704a` Add upstream sync helper script
3. `a00a78c` add Entity::field_paths and document entity snapshot tradeoffs
4. `0400de7` Merge branch 'Rupas1k:master' into on_entity_property_changed
5. `c273b4e` Use checked slice reader refill

### 当前分支落后上游的 5 个提交

1. `bd8af28` Add get_iter method for Entity (#3)
2. `651a7a0` Add get_property method for Entity
3. `a00e54b` Add unsafe feature
4. `027c165` Update examples
5. `1ff4309` Add public accessors for FieldValue

## 本次合并前落后上游 5 个提交的影响

### 1. `bd8af28` Add get_iter method for Entity (#3)

影响级别：低到中

判断：

- 这是 `Entity` API 的补充增强
- 和 `on_entity_property_changed` 的核心实现没有直接冲突
- 主要价值在于让外部调用方更方便遍历实体字段

对当前分支的意义：

- 与 `Entity::field_paths()` 的方向是互补的
- 即使不立即同步，也不会阻塞当前属性变更回调功能

### 2. `651a7a0` Add get_property method for Entity

影响级别：中

判断：

- 上游增加了更简洁的 `Entity::get_property(...)` 访问方式
- 这和你当前新增的 `Entity::get_property_by_field_path(...)` 不冲突
- 但会影响 examples、外部调用代码以及 API 习惯

对当前分支的意义：

- 建议同步
- 它能让“通过属性名访问”和“通过 FieldPath 访问”两条路径都更完整

### 3. `a00e54b` Add unsafe feature

影响级别：高

判断：

- 这是当前最需要关注的上游提交
- 它修改了：
  - `source2-demo/Cargo.toml`
  - `source2-demo/src/reader/slice.rs`
- 这两处恰好也是你当前分支已经改过的热点文件

对当前分支的意义：

- 你的分支当前是直接把 `SliceReader::refill()` 固定到 checked 路径
- 上游则把 unchecked 路径收敛成一个可选的 `unsafe` feature
- 如果后续同步上游，这里极可能发生冲突

建议：

- 优先回退本地 `slice.rs` 的自定义改法，改成上游的 `unsafe feature` 方案
- 如果仍需保留稳定性优先策略，应建立在上游 feature 设计之上，而不是继续长期分叉维护独立实现
- 目标是把这块维护成本降到最低

### 4. `027c165` Update examples

影响级别：低

判断：

- 只影响 examples 和依赖示例
- 不改变 parser 主体行为
- 对 `on_entity_property_changed` 主功能没有直接影响

对当前分支的意义：

- 可晚一点再同步
- 它更多是跟随上游 API 调整的示例修正

### 5. `1ff4309` Add public accessors for FieldValue

影响级别：中

判断：

- 这是对外 API 的增强
- 主要新增 `FieldValue` 的公开访问器，例如：
  - `string()`
  - `bool()`
  - `f32()`
  - `vec2()`
  - `vec3()`
  - `i32()` / `u32()` 等
- 同时会改动少量解码器内部调用

对当前分支的意义：

- 和你当前分支公开 `FieldPath`、新增 `get_property_by_field_path(...)` 是明显互补的
- 未来如果调用方在 `on_entity_property_changed` 回调里直接拿 `FieldValue` 读值，这组 accessor 会更顺手

建议：

- 建议同步
- 但优先级低于 `a00e54b`

## 以后分析“对比上游最新 master”时的默认流程

如果后续你只说“对比上游最新 master”，默认按下面流程执行：

1. 先确认比较口径

- 使用 `Rupas1k/source2-demo:master...azhezzzz:on_entity_property_changed`
- 不使用你 fork 内部的 `azhezzzz/source2-demo:master` 作为默认基线

2. 读取 GitHub compare 元数据

- 当前 `ahead_by`
- 当前 `behind_by`
- 上游 `master` 头提交
- 共同祖先提交

3. 列出上游新增提交

- 优先看当前分支落后于上游的提交
- 逐个记录提交标题、涉及文件和大致改动方向

4. 判断对当前分支的影响

- 是否直接冲突到以下热点文件：
  - `source2-demo/src/parser/demo/svc.rs`
  - `source2-demo/src/parser/observer.rs`
  - `source2-demo/src/reader/field/mod.rs`
  - `source2-demo/src/reader/slice.rs`
  - `source2-demo-macros/src/lib.rs`
  - `source2-demo/Cargo.toml`
- 是否会影响 `FieldPath`、`FieldValue`、`Entity` 公共 API
- 是否会影响 `on_entity_property_changed` 需求本身

5. 默认优先判断“能否回归上游”

- 上游已有等价实现：建议回退本地实现，改用上游
- 上游有相似实现：建议把本地实现改造成上游风格
- 上游没有覆盖：继续保留本地补丁

6. 输出建议时的默认优先级

- 先指出必须尽快处理的高冲突提交
- 再指出建议吸收、可减少维护成本的上游实现
- 最后说明哪些提交可延后处理

## 下游仓库迁移说明

这个分支已经被其他仓库消费时，后续同步上游后，默认也要给下游仓库一份迁移说明。

### 迁移目标

- 尽量减少下游对本分支私有实现细节的依赖
- 优先切换到上游已经提供的 API 和 feature
- 让下游后续升级到新版本时更平滑

### 这次合并上游后，下游应优先关注的迁移动作

#### 1. `SliceReader` 的稳定性策略改为基于上游 `unsafe` feature

如果下游仓库之前依赖的是本分支“强制 checked refill”的行为，应改为：

- 默认不启用 `unsafe` feature
- 只有在确认需要极限性能且可接受风险时，才显式开启 `unsafe`

也就是说，下游以后不应再假设这里是一个永久性的本地分叉行为，而应按上游 feature 语义理解。

#### 2. 优先使用上游 `Entity::get_property(...)`

如果下游代码还在使用：

- `get_property_by_name(...)`
- 自己封装的字符串属性访问 helper

建议逐步迁移到：

- `Entity::get_property(...)`

而 `get_property_by_field_path(...)` 则继续保留给 `on_entity_property_changed` 回调这种已知 `FieldPath` 的场景。

#### 3. 优先使用上游 `FieldValue` 公共访问器

如果下游之前依赖：

- `try_into()`
- 自己封装的值读取 helper

后续可逐步改用上游提供的访问器，例如：

- `f32()`
- `i32()`
- `u32()`
- `string()`
- `vec2()`
- `vec3()`

这能减少下游为 `FieldValue` 再包一层适配逻辑。

#### 4. 只把本分支特有 API 留给真正上游未覆盖的需求

当前本分支仍然需要重点保留的自定义能力主要是：

- `on_entity_property_changed`
- `FieldPath` 对外暴露后的按属性粒度通知链路
- `Entity::field_paths()`
- `Entity::get_property_by_field_path(...)`

如果上游未来补了这些能力或提供了等价接口，应优先继续回退本地实现并迁移下游调用。

### 每次同步上游后，默认要给下游仓库输出的内容

1. 当前从哪个提交升级到了哪个提交
2. 是否有 feature 语义变化
3. 是否有 API 可从本地实现切换为上游实现
4. 下游需要改哪些调用点
5. 是否需要重新跑解析验证或回归测试

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
