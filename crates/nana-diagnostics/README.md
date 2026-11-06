# nana-diagnostics

低开销的结构化诊断：事件、指标、故障、内存里的 flight recorder，以及 `.nlog` 文件格式。

调用点用 `event!`、`metric!`、`fault!` 和 `span!`，搭配静态描述符。事件写进线程本地的环形缓冲，满了就丢；后台 worker 把它们批量写进 `.nlog`，同时在内存里保留最近一分钟左右的记录，panic 时由 `install_panic_hook` 写出快照。生产线程上没有文件 I/O、锁和格式化，没有安装实例时每个调用点只是一次原子读取。NanaUI 自己的描述符在 `framework` 模块里，域 `0x0000..=0x00FF` 归框架保留，应用从 `Domain::APPLICATION_MIN` 开始。

这个 crate 不依赖其它 NanaUI crate。日志和崩溃目录由 [`nana-ui-platform`](../nana-ui-platform) 的 `ApplicationPaths` 解析后传进来，脱敏和上报由应用决定。

## 示例

```rust
use nana_diagnostics::{
    Domain, EventDescriptor, FieldDescriptor, HistogramCells, Metric, Severity, event, metric,
};

const LIVE: Domain = Domain(0x0100);
static MODEL_LOADED: EventDescriptor = EventDescriptor::new(
    LIVE, 1, "live.model_loaded", Severity::Info, &[FieldDescriptor::u64("bytes")],
);
static TRACK_CELLS: HistogramCells = HistogramCells::new();
static TRACK_NS: Metric = Metric::histogram(LIVE, 1, "live.track", "ns", &TRACK_CELLS);

event!(MODEL_LOADED, bytes = 1024u64);
metric!(TRACK_NS, std::time::Duration::from_micros(250));
```

查看 `.nlog` 用自带的 `nana-nlog`：

```bash
cargo run -p nana-diagnostics --bin nana-nlog -- text <file.nlog>
```

## 测试

```bash
cargo test -p nana-diagnostics
```

热路径基准和其余检查见 [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml)。

更多内容见 [诊断与应用路径](../../docs/reference/diagnostics.md)。
