# iced layout markup language & hotswap test

This was created for https://github.com/hecrj/iced/issues/21

`cargo run`

## todo:

- [x] Templated layout engine
  - `ron`-based markup language
  - Templates handled with handlebars
- [x] Handle user-defined messages for callbacks
  - User-defined message type must be `serde::Deserialize`
- [x] User-defined state
- [x] Hot-reloading
  - Missing a `Subscription` that watches for file changes
- [ ] Implement all iced_native widgets
- [ ] Custom widgets?
- [ ] fix std::mem::transmute horrors
