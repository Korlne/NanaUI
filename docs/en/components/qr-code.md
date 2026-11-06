# QrCode

`QrCode` draws a piece of data as a scannable QR code. It is not focusable, and its accessibility role is image. The default name is "QR code", and an empty label falls back to it.

## Basic usage

There is no `<QrCode>` in the control table.

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::QrCode;

let code = QrCode::encode(b"nana://join", QrCode::DEFAULT_SIZE)?.label("Join");
view! {
    <Widget of={code} />
}
```

```rust rust
use nana_ui::runtime::view::widget;
use nana_ui::runtime::QrCode;

let code = QrCode::encode(b"nana://join", QrCode::DEFAULT_SIZE)?.label("Join");
widget(code)
```

:::

The first argument of `encode` is bytes. Turn a string into bytes first. The result is a `Result`; on failure, replace this image with your own note.

## Encoding

`QrCode::encode(data, size)` encodes the payload with dark modules as true, then hands it to painting. A failed encode returns `EncodeFailed`.

## Pre-arranged modules

`QrCode::from_modules(modules, width, size)` takes a matrix that is already arranged. When the matrix is empty, the side length is 0, or the length is not `width * width`, it returns `QrCodeError::InvalidModules`.

## Size

A non-finite size returns `NonFiniteSize`. `DEFAULT_SIZE` is 224 and `MIN_SIZE` is 64; a smaller or non-finite size is raised to 64. `.size` changes the side length again.

## Quiet zone

The quiet zone is 4 modules on each side, added at paint time and not written into the matrix.

## Module geometry

Module pixels and origin are computed by `module_geometry` from the layout box: take the shorter of width and height, divide by the module count plus 8, round down, take at least 1, then center in the box.

## Pointer

The QR code itself does not take the pointer. When the card's empty space should also belong to the card, use `Stack::column(...).hittable()` outside.

## Properties

`.label` replaces the accessible name. `.size` changes the side length again.

| Property | Type | Description |
| --- | --- | --- |
| `.label` | — | Replaces the accessible name. The default is "QR code", and an empty label falls back to it |
| `.size` | — | Changes the side length again. `DEFAULT_SIZE` is 224 and `MIN_SIZE` is 64; a smaller or non-finite size is raised to 64 |

## Events

| Event | Payload | Description |
| --- | --- | --- |
| None | None | None |

## Slots

| Slot | Description |
| --- | --- |
| None | None |

## See also

[Overview](https://github.com/sena-nana/NanaUI/blob/main/docs/components/index.md) · [Controls](https://github.com/sena-nana/NanaUI/blob/main/docs/reference/components.md)
