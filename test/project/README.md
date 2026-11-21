# Harbor 采集服务

Harbor 接收设备事件。仓库内的 converter 将供应商数据转换为统一事件，reporter 根据模板生成日报。

这个目录是 LianComment 的手动测试样例。可先复制到临时目录，再用 CLI 初始化项目：

```bash
lc init
lc set config.toml "采集服务监听配置"
lc set deploy "服务部署配置"
(cd tools/converter && lc init)
(cd tools/converter/plugins/csv && lc init)
(cd tools/reporter && lc init)
lc set tools/converter/src/convert.py "事件转换入口"
lc set tools/converter/plugins/csv/samples/devices.csv "供应商样例"
lc set tools/reporter/templates/daily.txt "日报模板"
```

- 在仓库根运行 `lc -R`，应看到普通目录中的内容，并在 converter 和 reporter 两个项目根停止递归。
- 在 `tools/converter` 运行 `lc -R`，应看到 `src/convert.py` 的注释，并在 CSV 子项目根停止递归。
- 在 `tools/converter/plugins/csv/samples` 运行 `lc`，应看到 `devices.csv` 的注释。
- 在仓库根运行 `lc -a`，应能看到 `.github/`。
- 从根目录导出，再在另一份样例中只初始化外层项目。导入包后，应创建嵌套清单并恢复各级注释。
- 在仓库根确认执行 `lc clear --all` 后，嵌套项目的注释应仍然可查。
