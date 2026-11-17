# 发布到 Scoop 与 Winget 指南

## 1. 触发 GitHub Release 构建

在仓库初始化并推送到 GitHub（例如 `https://github.com/huxulm/lcctl`）后，打 Tag 即可自动触发 [release.yml](../../.github/workflows/release.yml) 构建各平台产物并生成 `SHA256SUMS.txt`：

```bash
git tag v0.1.0
git push origin v0.1.0
```

Windows 产物文件名约定：
`lcctl-v0.1.0-x86_64-pc-windows-msvc.zip`（包含 `lcctl.exe`、`LICENSE`、`README.md`）。

---

## 2. 发布到 Scoop（个人 Bucket）

1. 在 GitHub 新建仓库 `scoop-bucket`（或任意名称），在其中创建 `bucket/lcctl.json`。
2. 复制 [lcctl.json](../scoop/lcctl.json) 到该仓库，并将 `hash` 替换为 Release 中 `SHA256SUMS.txt` 对应的 SHA256 值。
3. 用户即可安装：
   ```powershell
   scoop bucket add huxulm https://github.com/huxulm/scoop-bucket
   scoop install lcctl
   ```

---

## 3. 发布到 Winget（`microsoft/winget-pkgs`）

Winget 原生支持 Portable ZIP 格式，无需制作 `.msi` 安装包：

1. 安装官方生成工具：
   ```powershell
   winget install Microsoft.WingetCreate
   ```
2. 根据 Release 下载链接生成 Manifest 并提交 PR：
   ```powershell
   wingetcreate new https://github.com/huxulm/lcctl/releases/download/v0.1.0/lcctl-v0.1.0-x86_64-pc-windows-msvc.zip
   ```
3. 按交互提示确认：
   - PackageIdentifier：`Huxulm.lcctl`
   - InstallerType：`zip` / NestedInstallerType：`portable`
   - RelativeFilePath：`lcctl.exe`
   - PortableCommandAlias：`lcctl`
   - License：`MIT`
4. 后续发布新版本时直接执行：
   ```powershell
   wingetcreate update Huxulm.lcctl -v 0.2.0 -u https://github.com/huxulm/lcctl/releases/download/v0.2.0/lcctl-v0.2.0-x86_64-pc-windows-msvc.zip --submit
   ```
