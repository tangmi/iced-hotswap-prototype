# iced layout markup language & hotswap test

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
