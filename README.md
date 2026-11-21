# LianComment

LianComment（`lc`）是一个面向 Linux 的项目级文件注释工具

有一天啊, 我在 Linux 终端里整理目录, 因为我是一个极度的精神洁癖! 我必须搞明白我家目录里的文件都是怎么出现的! 根目录, `.local`, `.cache`, `.config`...

但是呢, 给每个目录写一份说明又太麻烦了...

喂喂 Linux! 你不是宣称什么“万物皆文件”吗? 能不能给你的每一个文件都加上一段注释或描述啊? 这样我在终端里用一条命令就能看到每个文件是干嘛的, 不用打开 `nvim` 看内容, 也不用到处留 `README.md`, 不就方便了吗!

考虑了很多种存储方案（比如 `xattr`、`sqlite`）之后, 我从 `uv` 和 `git` 那儿得到了点启发, 决定写一个命令行工具, 用项目里一个简单的 JSON 文件解决这个问题。

---

## 快速上手

这里帮你快速了解 `lc` 能干什么! 在仓库根目录运行 `cargo install --path .`，然后进入一个含有 `config.toml` 的项目目录试试。

```bash
lc init
lc set config.toml "运行时配置"
lc edit config.toml
```

`init` 在当前目录创建 `.lc.json`。`set` 写一句简短注释，`edit` 打开编辑器，可以接着写详细描述。

```console
$ lc
Name          Comment
────────────────────────
config.toml   运行时配置
README.md

$ lc show config.toml
Path         config.toml
Type         File

Comment
  运行时配置

Description
  配置服务监听地址和日志级别。
  修改后重启服务生效。
```

---

## 文档

- [命令帮助](docs/commands.md)
- [项目规则](docs/project-rules.md)

## MIT

项目采用 [MIT 许可证](LICENSE)。
