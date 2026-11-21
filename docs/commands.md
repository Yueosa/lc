# LianComment 命令帮助

本文介绍 CLI 命令和示例。项目介绍见 [README](../README.md)，存储规则见 [项目规则](project-rules.md)。

## 初始化

在项目根目录运行 `lc init` 创建 `.lc.json`。其余命令通过 [项目边界规则](project-rules.md#项目边界) 选择清单。命令参数可使用相对路径或绝对路径，清单中的路径相对于项目根。

## 浏览项目

### 默认列表

直接运行 `lc`，列出当前目录中的文件、目录及其注释。

```console
$ lc

Name                 Comment
────────────────────────────────────────────────────────
src/               核心业务逻辑
assets/            静态资源文件
scripts/           自动化脚本
Cargo.toml         Rust 项目配置
Cargo.lock         依赖版本锁定文件
README.md          项目说明文档
LICENSE
```

没有注释的文件仍然正常显示，注释列留空。

默认按照文件名排序，目录优先，不显示隐藏文件。

目录名后带有 `/` 后缀。

### 显示隐藏文件

```console
$ lc -a

Name                 Comment
────────────────────────────────────────────────────────
.git/               Git 版本控制数据
.github/            GitHub 工作流配置
src/                核心业务逻辑
assets/             静态资源文件
.gitignore          Git 忽略规则
Cargo.toml          Rust 项目配置
README.md           项目说明文档
```

### 详细列表

`-l` 显示权限、大小、修改时间及注释。

```console
$ lc -l

Permissions   Size     Modified          Name          Comment
────────────────────────────────────────────────────────────────────────────
drwxr-xr-x    4.0K     2026-11-20 18:30  src/          核心业务逻辑
drwxr-xr-x    4.0K     2026-11-19 12:00  assets/       静态资源文件
-rw-r--r--    1.2K     2026-11-20 17:20  Cargo.toml    Rust 项目配置
-rw-r--r--    8.4K     2026-11-18 09:10  README.md     项目说明文档
```

支持组合参数：

```bash
lc -la
lc -al
lc -la ~/projects/lianbot
```

### 递归列表

```console
$ lc -R

./
├── src/                 核心业务逻辑
│   ├── api/             HTTP 接口
│   │   └── mod.rs       接口模块入口
│   ├── runtime/         任务调度与生命周期管理
│   └── main.rs          程序入口
├── assets/              静态资源
│   └── logo.svg         项目 Logo
└── Cargo.toml           Rust 项目配置
```

递归输出采用树形结构。

默认不跟随符号链接进入其他目录。符号链接条目的注释属于链接自身。

## 注释与描述

每个文件或目录可以拥有两种独立的信息。

**Comment：简短注释**

用于快速说明文件用途，主要显示在目录列表中。

**Description：详细描述**

用于记录设计意图、维护注意事项或其他较长的说明，支持多行文本。

两者均为可选内容。

### 设置注释

```bash
lc set src/ "项目核心业务逻辑"
lc set Cargo.toml "Rust 项目配置"
lc set README.md "项目说明文档"
```

成功时：

```console
$ lc set src/ "项目核心业务逻辑"

✓ Comment updated: src/
```

已有短注释且内容不同时，命令会显示旧值和新值，并询问 `Overwrite? [y/N]`。按回车或拒绝时保留旧值。内容相同则直接返回。脚本可以传 `--force` 显式覆盖，非交互模式遇到冲突时返回错误。

### 查看注释

```console
$ lc get src/

项目核心业务逻辑
```

`get` 默认只输出注释正文，方便配合 Shell 脚本使用。

### 在编辑器中编辑注释与描述

```bash
lc edit src/runtime/
```

打开用户默认的文本编辑器，并预填 Comment 和 Description。可以只改一项，也可以同时修改。Comment 为单行，Description 可以包含多行。编辑器正常退出后，命令校验内容并一次性保存。

编辑完成并保存后：

```console
✓ Annotations updated: src/runtime/
```

### 查看完整信息

```console
$ lc show src/runtime/

Path         src/runtime/
Type         Directory

Comment
  任务调度与生命周期管理

Description
  负责后台任务的创建、调度和生命周期管理。

  该模块不应包含具体业务逻辑。
  所有任务必须支持取消和资源回收。
```

`show` 同时展示文件信息、简短注释和详细描述。

未设置的字段显示为 `(not set)`。

## 删除与清理

### 删除指定文件的注释

```console
$ lc rm src/main.rs

✓ Annotations removed: src/main.rs
```

同时删除 Comment 和 Description。

不会删除文件本身。

### 清空指定目录的所有注释

```console
$ lc clear ./project

Target             /home/user/project
Annotated entries  128
Comments           103
Descriptions       42

This will permanently remove all annotations
from the target directory and its descendants.

Type "confirm" to continue: confirm

✓ Cleared 128 annotated entries.
```

清理范围包含目标目录自身及同一项目内的后代路径，也包含范围内已缺失路径的记录。遇到嵌套项目时停止。

必须输入完整的 `confirm` 才会继续。

其他输入或直接回车均取消操作。

```console
Type "confirm" to continue: no

Operation cancelled.
```

### 清空当前项目的注释

```bash
lc clear --all
```

该命令清空当前项目边界内的全部注释和描述。可从项目内任意目录执行。

执行前必须明确显示清理范围及受影响数量。

```console
$ lc clear --all

WARNING: Project annotation cleanup

Project root       /home/user/project
Annotated entries  2,418

This operation cannot be undone.

Type "confirm" to continue: _
```

如果无法完整确定清理范围，命令必须报错，而不是执行不完整或范围不明确的清理。

### 清理失效记录

```console
$ lc clean ~/projects

Invalid entries found: 3

Status      Path
────────────────────────────────────────────────────────
Missing     ./old-project/config.toml
Mismatch    ./demo/state.db
Missing     ./archive/notes.md

Remove these invalid records? [y/N]: y

✓ Removed 3 invalid records.
```

`clean` 只处理已经确认失效的注释记录。

无法验证的记录不自动删除。

## 文件有效性检查

注释描述项目中某个路径的用途。同一路径上的文件被编辑器原子替换后，只要类型仍符合预期，注释就继续有效。清单记录预期类型。类型变化时标记为 `Mismatch`，并隐藏注释正文。

使用外部命令改名或移动文件后，清单仍保留原路径。原路径缺失时标记为 `Missing`，用户可以在清单中显式更新记录的相对路径。若同类型的文件后来占用原路径，它会继承该路径的注释。

### 检查指定路径

```console
$ lc check ~/projects

Checked entries: 156

Valid         151
Missing         2
Mismatch        1
Unverified      2
```

支持以下状态：

| 状态 | 含义 |
|---|---|
| `Valid` | 路径存在且类型符合预期 |
| `Missing` | 记录中的路径不存在 |
| `Mismatch` | 路径存在，但类型与记录不符 |
| `Unverified` | 因权限或其他读取错误无法验证路径或类型 |
| `Unsupported` | 无法在当前环境完成相关操作 |

对于检测到异常的记录，普通列表中应明确标记。

```console
$ lc -a

Name                 Comment
────────────────────────────────────────────────────────
src/                 核心业务逻辑
config.toml          运行时配置
state.db             [Mismatch]
README.md            项目说明文档
```

异常记录不应直接显示可能属于旧文件的注释。

`lc check` 只检查，不修改或删除任何记录。

## 导入与导出

### 导出注释

```console
$ lc export ./project -o annotations.json

✓ Export completed.

Entries exported: 42
Output: annotations.json
```

导出默认包含指定目录自身及所有后代路径的注释和描述，包括隐藏目录与带有独立 `.lc.json` 的嵌套项目。遍历时不跟随目录符号链接。输出路径已有文件时，命令报错并保留原文件。

导出包按项目分组。指定目录所属项目的记录只取指定目录覆盖的部分，记录路径以指定目录为基准。每个嵌套项目保留其根目录相对于导出目录的位置，组内路径相对于各自的项目根。

### 导入注释

```console
$ lc import annotations.json --root ./project

Entries found:    42
New annotations:  38
Conflicts:         3
Missing targets:   1

Import 38 new annotations? [y/N]: y

✓ Imported 38 entries.
```

默认不覆盖已有注释。

`--root` 指定导入路径的基准目录，该目录须已属于某个项目。导入将主组记录写入该项目的清单，并按导出包记录的位置恢复各嵌套项目的清单。嵌套项目目录已经存在时，可以创建缺失的 `.lc.json`。导入前会列出计划创建或更新的清单，确认后再写入。每份清单独立提交；如果中途遇到写入错误，已经提交的项目会保留，可以重新运行导入继续处理。

路径不存在、类型不符、已有注释或与目标项目边界冲突的条目会被跳过并报告。导入拒绝绝对路径和任何试图越过目标目录边界的路径。

## 命令速查

| 命令 | 作用 |
|---|---|
| `lc init` | 在当前目录创建项目清单 |
| `lc [path]` | 浏览文件及注释 |
| `lc -a` | 显示隐藏文件 |
| `lc -l` | 显示详细列表 |
| `lc -R` | 递归显示目录树 |
| `lc set <path> <text> [--force]` | 设置简短注释 |
| `lc get <path>` | 获取简短注释 |
| `lc edit <path>` | 在编辑器中编辑注释与描述 |
| `lc show <path>` | 查看完整信息 |
| `lc rm <path>` | 删除指定文件的注释与描述 |
| `lc clear <path>` | 递归清空指定目录的全部注释 |
| `lc clear --all` | 清空当前项目的注释 |
| `lc check <path>` | 检查注释关联有效性 |
| `lc clean <path>` | 清理失效记录 |
| `lc export <path> -o <file>` | 导出目录及其嵌套项目的注释 |
| `lc import <file> --root <dir>` | 导入注释并恢复嵌套项目清单 |
| `lc --help` | 显示帮助 |
| `lc --version` | 显示版本 |

## 交互规范

**输出原则**

- 普通查询以可读性为主。
- 简短查询不输出多余信息。
- 列表列宽根据终端宽度自适应。
- 长注释可以截断，但不修改原始内容。
- 终端支持颜色时，对目录、注释和异常状态使用不同样式。
- 非交互式输出不依赖颜色和特殊图标。
- 错误信息应明确指出目标与失败原因。

**安全原则**

- 所有注释操作均不修改文件正文。
- 删除注释不会删除实际文件。
- 批量清理需要二次确认。
- 不自动删除无法验证的记录。
- 不自动猜测已改名条目的新路径。
- 不默认跟随符号链接进行递归操作。
