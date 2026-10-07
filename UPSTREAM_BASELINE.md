# 上游比较基线

本文档只保存下一次上游更新分析必须使用的当前基线。每次成功合并新的
`origin/master` 后都必须更新本文件，不在这里堆叠历史记录。

- 最近更新时间：`2026-10-07`
- 当前已合并的 `origin/master`：`b1b1d504a29267e15b4d925126ddfa1f1e6b746d`
- 对应上游版本：`0.5.9`
- merge commit：`2b0e15c1117ca0226cf1917233eca56e6b249498`
- 当前定制分支记录 commit：`1d34dd4`

下一次用户在 GitHub 同步 fork `master` 后，先执行 `git fetch origin`，再比较：

```bash
git log --oneline b1b1d504a29267e15b4d925126ddfa1f1e6b746d..origin/master
git diff --stat b1b1d504a29267e15b4d925126ddfa1f1e6b746d..origin/master
git diff --name-status b1b1d504a29267e15b4d925126ddfa1f1e6b746d..origin/master
```

不要使用当前 `on_entity_property_changed` 的 HEAD 代替上面的 `origin/master` 基线。
