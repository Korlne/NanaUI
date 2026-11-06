# Dropdown

Single-value or multi-value selection. It shares the field surface and menu with [Select](select.md). To search as you type, use [SearchDropdown](search-dropdown.md).

There is no `<Dropdown>` in the control table. Both authoring styles call `widget`.

Option identity belongs to your application. Disabled options stay visible; they just cannot be selected.

## Basic usage

Use `Dropdown::single(value)` for single selection and `Dropdown::multiple(values)` for multiple selection. The real `new` is private and takes a `DropdownSelection`; do not invent your own `Dropdown::new`.

:::api

```rust view
use nana_ui::runtime::view;
use nana_ui::runtime::view::widget;
use nana_ui::runtime::Dropdown;

widget(Dropdown::single(value))
```

```rust rust
use nana_ui::runtime::view::widget;
use nana_ui::runtime::Dropdown;

widget(Dropdown::single(value))
```

:::

Options use `DropdownOption::new(value, label)` and can take `.hint` and `.disabled`. In the menu, `hint` is drawn at the end of the row, one size smaller than the label and in the secondary color, the same as option hints in [Select](select.md).

## After selection

When the user selects, the control writes the selected value itself and closes the menu.

The single-value event is `DropdownEvent::Select`, the multi-value event is `DropdownEvent::Toggle`, and open and close are `Opened` and `Closed`. While disabled or loading, `toggle_open` emits no event. `inactive` is true while disabled or loading.

## Screen readers

The open menu works like [Select's screen reader support](select.md#screen-reader): a list box under the field. With multiple selection the list box is multi-selectable (`aria-multiselectable`), and every checked item is "selected". Activating an item from a screen reader checks or unchecks it and leaves the menu open, the same as pressing the row.

## Properties

| Property | Type | Description |
| --- | --- | --- |
| Selected value | Argument of `Dropdown::single` / `Dropdown::multiple` | One value for single, a list for multiple. Do not call the private `Dropdown::new` |
| Options | `DropdownOption` | `DropdownOption::new(value, label)`, optionally followed by `.hint` and `.disabled` |
| `placeholder` | field | Field surface shared with Select |
| `size` | field | Same as above |
| `disabled` | field | While disabled, `toggle_open` emits no event |
| `loading` | field | While loading, `toggle_open` emits no event |
| `invalid` | field | Validation state |
| `opened` | field | Whether the menu is open |
| `highlighted` | field | Currently highlighted item |
| `menu_scroll` | field | Menu scroll position. For menu expansion, scrolling, and the scrollbar, see [Select's menu](select.md#menu) |
| `bare_trigger` | field | Drops the fill and border at rest, for use as a picker on a toolbar; hover and the open state still get a background, and the validation state still draws a red border |

## Events

| Event | Payload | Description |
| --- | --- | --- |
| `DropdownEvent::Select` | single-value commit | The user selected an item |
| `DropdownEvent::Toggle` | multi-value toggle | One item changed in a multiple selection |
| `Opened` / `Closed` | open state | The menu opened or closed |

## Slots

No named slots. Options are data, not child nodes.

## See also

[Overview](index.md) · [Select](select.md) · [Controls](../reference/components.md)
