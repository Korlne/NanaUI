# Fetch

Network permission belongs to the host, not the UI. By default no origin is allowed. List the allowed origins first; only then can remote images in styles be fetched.

```rust
use nana_ui::{ApplicationWindow, FetchPolicy, NativeFetchHost, shared_fetch_host};

let policy = FetchPolicy::default().with_allowed_origin("https://cdn.example.com")?;
let mut window = ApplicationWindow::default();
window.fetch_host = Some(shared_fetch_host(NativeFetchHost::new(policy)));
```

`FetchPolicy`, `NativeFetchHost` and `shared_fetch_host` are re-exported from `nana_ui` when `gpu` is enabled; they are defined in `nana_ui_platform`.

## Allowlist and limits

Origins are registered with `allow_origin` / `with_allowed_origin`, in the form `scheme://host[:port]`. A path, query or fragment yields `InvalidRequest`. Anything other than http / https yields `Policy`. Allowing `https://example.com` allows every path under it. `https://api.example.com` is a different origin and must be registered separately. localhost must be allowed separately too.

| Field | Default |
| --- | --- |
| `timeout` | 30 seconds, shared across redirects |
| `max_request_bytes` | 16 MiB (`16 * 1024 * 1024`) |
| `max_response_bytes` | 16 MiB, counted cumulatively for streams |
| `max_redirects` | 5; exceeding it yields `Redirect` |

`NativeFetchHost` is blocking `ureq` and must run off the UI thread; the painter calls it on its own image-fetch thread. Every hop is re-checked against the allowlist. Cross-origin hops strip `authorization` and `proxy-authorization`. A 303, and a 301 / 302 for POST, become GET, with the body and `content-length` / `content-type` cleared. `set-cookie` in responses is dropped; there is no cookie jar.

Errors are a `FetchErrorKind`: `Policy`, `InvalidRequest`, `Network`, `Timeout`, `RequestTooLarge`, `ResponseTooLarge`, `Redirect`, `Cancelled`, `Unsupported`.

## Which resources go through this path

`url()` images, `<img src>`, `mask-image` and `border-image` share one `FetchHost` and one `FetchPolicy`: the same allowlist, the same per-hop redirect checks, the same stripping of auth headers across origins.

Without a host installed, remote images are not fetched. `data:`, `file:` and relative paths inside the jail are unaffected. Hosts are per document. When several windows share one painter, each document's images go only through its own host, and caches are also kept per host.

A window's default is `None`, and remote images are rejected. Fill in `ApplicationWindow.fetch_host`, or implement `RuntimeProgram::resource_fetch_host`. If you own the painter, hand the document's host over with `SceneWgpuPainter::set_resource_fetch_host` before painting that document.

Cancellation happens when the painter is dropped, and when an image has gone unreferenced for 120 consecutive frames. It ends the network wait, not a decode that has already started. `@font-face` does not use the network; it accepts only `local()`, `data:` and local files inside the jail.

## Implementing your own host

The host interface is `FetchHost`: implement `fetch` and `policy()`. `policy()` declares this host's own allowlist and limits.

The host contract is in [Application API](../../reference/application-api.md#fetch-host).

## Next steps

<div class="next-steps">
  <a class="next-step" href="/guide/components/async">
    <p class="next-step-link">Async</p>
    <p class="next-step-caption">A resource in a Rust view runs on the UI thread's executor.</p>
  </a>
  <a class="next-step" href="/guide/scaling/packaging">
    <p class="next-step-link">Packaging</p>
    <p class="next-step-caption">nana://res in styles reads from resource packs.</p>
  </a>
  <a class="next-step" href="/guide/scaling/accessibility">
    <p class="next-step-link">Accessibility</p>
    <p class="next-step-caption">Names, hidden subtrees and the window root.</p>
  </a>
</div>
