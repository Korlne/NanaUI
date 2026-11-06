# nana-packager

Nana 应用的构建期打包工具，提供 `nana-packager` 命令和同名库。

它读取 `nana-package.toml` 和已经构建好的可执行文件，把资源打成 `.nrpack`，按目标平台组装目录布局，写出包清单和发布者签名，也能生成 Steam depot 脚本。给出上一版产物时，未变化条目保持原来的偏移，SteamPipe 的增量补丁因此更小。`validate` 对交付目录做最终检查，跑不了的检查报告为 `not-executed`，不会算作通过。

它不调用 cargo，也不进入应用二进制，运行时读取资源包的是 [`nana-package`](../nana-package)。平台签名（codesign、公证、Authenticode）还没有内置实现。

## 示例

```bash
nana-packager keygen content --out content.key
nana-packager package --config nana-package.toml --exe target/dist/my-app --out out
nana-packager validate out/app/<应用根> --trust-key ed25519:HEX --run
```

`nana-packager inspect` 和 `nana-packager delta` 用来查看资源包和估算两版之间的差异。完整参数见 `src/main.rs` 的 `USAGE`。

## 测试

```bash
cargo test -p nana-packager
```

端到端的打包和防篡改流程见 [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml) 的 `packaging` 任务。

更多内容见 [打包与分发](../../docs/reference/packaging.md) 和 [打包指南](../../docs/guide/scaling/packaging.md)。
