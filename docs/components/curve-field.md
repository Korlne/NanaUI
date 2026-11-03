# CurveField

单位方格里的一条折线，线上的点横向位置固定，只能上下拖动改高度。用来编辑映射曲线、响应曲线这类“输入 0–1 对应输出 0–1”的形状。模板里的标签是 `<Curve>`，函数是 `curve(step)`，类型是 `CurveField`。`step` 是 `f64`，高度落在它的格子上。

## 基本用法

:::api

```rust view
use nana_ui::runtime::view;

view! {
    <Curve step=0.001 ends={Some((0.0, 1.0))} label="映射曲线"
        point_labels={names} v-model={heights} />
}
```

```rust rust
use nana_ui::runtime::view::curve;

curve(0.001)
    .ends(Some((0.0, 1.0)))
    .label("映射曲线")
    .point_labels(names)
    .model(heights)
```

:::

`heights` 是 `Signal<Vec<f64>>`，每个点一个高度，从左到右，都在 `0..=1`。点的个数就是它的长度：换一组不同长度的值，点会重新装配。`v-model={heights}` 展开成 `.model(heights)`，听的是 `CurveInput`，拖动过程中的预览也会写回。`names` 是 `Vec<Arc<str>>`，各点的读屏名字。

不用模板时直接构造：`CurveField::new(vec![0.2, 0.4, 0.6], 0.001).ends(0.0, 1.0)`。

## 点的位置

`n` 个点里第 `k` 个（从 0 数）在 `x = (k + 1) / (n + 1)`，在两端之间均匀排开；`position(k)` 给出这个值。点只上下动，拖动时指针往旁边偏不会换点。

`ends` 是曲线在 `x = 0` 和 `x = 1` 的固定高度：画出来的线从左端起、经过每个点、到右端止。端点不画手柄、不收输入。没有 `ends` 时线从第一个点画到最后一个点。

## 尺寸和内缩

宽度默认铺满，没给高度时按 1:1 的宽高比成正方形；要别的比例就自己给高度或 `aspect_ratio`。曲线的方格不在节点的边上，四边各往里缩 `curve_plot_inset(size)`（实例上是 `plot_inset()`）：半个点加焦点环，点停在角上时焦点环仍在节点里。要在旁边画刻度或实时值，按它对齐：点 `(x, y)` 在节点里的位置是 `(inset + x * (宽 - 2 * inset), inset + (1 - y) * (高 - 2 * inset))`。

## 外观

方格有一圈 `Border` 细框和四等分的 `BorderSoft` 网格线，曲线是一条强调色（`Accent`）折线，拐角圆接。点与 [RangeField](range-field.md) 的滑块同样，直径小一档（`curve_point_extent(size)`）。禁用时曲线转 `Faint`，点转禁用色。点动了只换方格节点的绘制键和那个点的位置。

## 可见取值和提交

`CurveInput { point, values }` 是每一次可见取值，含拖拽中的预览。`CurveChanged { point, values }` 是提交：指针抬起且点动了、键盘步进、无障碍的增减或设值，或 `set_curve_point` / `set_curve_values`。`point` 是动的那个点；整组替换时是 `None`。取消的拖拽不提交，并恢复按下前的高度。拖动起止另有 `CurveDragging { dragging }`，键盘和无障碍步进不算拖动。三者与 [RangeSpanField](range-span-field.md) 的 `RangeSpanInput` / `RangeSpanChanged` / `RangeSpanDragging` 一一对应。

## 指针

按在点上就抓住这个点，不会跳到指针下面。按在方格别处，横向最近的点移到指针的高度（正好在两点中间时取左边那个）。拖动期间控件捕获指针，移动的点拿到焦点。高度夹在 `0..=1`。

## 键盘和读屏

每个点是一个焦点停留点，Tab 从左往右走。焦点在点上时，左右方向键换到相邻的点（到头时停在原处，这一下仍被控件收下）；上下方向键按 `step` 改高度，PageUp / PageDown 按 `page_step`（默认 `step` 的十倍），Home / End 到 0 和 1。焦点可见时点外有一圈焦点环。

每个点在读屏里是一个竖向的 `Slider`：名字取 `point_labels` 里对应的一项，没有就是它的序号（“1”“2”…）；值读作百分比（`35%`，小数位跟着步进），数值范围是 0 到 1。支持 `Increment`、`Decrement` 和 `SetValue`，`SetValue` 收 `"35%"` 也收 `"0.35"`。整个控件是一个带 `label` 的分组。点的节点用 `point_node(k)` 拿到。

## 边界

`CurveField::new` 修不一致的输入：非有限或不为正的步进改成 `0.01`，大于 1 的步进按 1；每个高度夹进 `0..=1` 并对齐步进，非有限的高度是 0。步进是短小数时，值正好落在格子上（`0.6`，不是 `0.6000000000000001`）。`set_curve_values` / `set_curve_point` 遇到非有限的值返回错误。

## 属性

| 属性 | 类型 | 说明 |
| --- | --- | --- |
| `values` | `Vec<f64>` | 各点高度，从左到右。模板写入时夹紧并对齐步进，个数决定点数。`v-model` 绑它，来源是 `CurveInput` |
| `step` | `f64` | `curve` 的参数 |
| `ends` | `Option<(f64, f64)>` | 曲线在两端的固定高度 |
| `label` | `Option<Arc<str>>` | 分组名 |
| `point_labels` | `Vec<Arc<str>>` | 各点的读屏名字 |
| `disabled` | `bool` | 点不可聚焦、不收指针，曲线和点转为禁用色 |
| `page_step` | — | 不在模板字段表里。翻页步进，默认 `step` 的十倍 |
| `size` | — | 不在模板字段表里。`ControlSize`，决定点的直径和内缩 |

## 事件

| 事件 | 载荷 | 说明 |
| --- | --- | --- |
| `on_input`，模板 `on:CurveInput` 或 `@input` | `&CurveInput` | 每一次可见取值，含拖拽中的预览 |
| `on_change`，模板 `on:CurveChanged` 或 `@change` | `&CurveChanged` | 提交：抬起且动了、键盘步进、无障碍增减和设值、程序设置。取消的拖拽不提交 |
| `.on` 听 `CurveDragging` | `&CurveDragging` | `{ dragging }`。拖动起止 |

## 程序接口

| 方法 | 说明 |
| --- | --- |
| `set_curve_values(field, values)` | 提交整组高度，个数可以变 |
| `set_curve_point(field, k, value)` | 提交一个点的高度 |
| `adjust_curve_point(field, k, RangeAdjustment)` | 像按键一样步进一个点 |

## 插槽

| 插槽 | 说明 |
| --- | --- |
| 无 | 点的节点由控件自己装配 |

控件没有登记到组件注册表；在 Rust 和 `view!` 里用。

## 参见

[总览](index.md) · [RangeSpanField](range-span-field.md) · [RangeField](range-field.md) · [TimelineEditor](timeline-editor.md)
