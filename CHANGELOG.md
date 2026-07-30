# Changelog

## [0.1.31](https://github.com/mcthesw/easy-nats/compare/v0.1.30...v0.1.31) (2026-07-30)


### Features

* **app:** add browser demo runtime ([5a5f8f0](https://github.com/mcthesw/easy-nats/commit/5a5f8f029ddd790c08af2ef60e1123c1e8e2c017))
* **app:** expose web demo preferences ([ec28f2a](https://github.com/mcthesw/easy-nats/commit/ec28f2a4152eec7f6bc44a61a2a132eed9f30006))
* **backend:** add in-memory demo backend ([4551d65](https://github.com/mcthesw/easy-nats/commit/4551d659d75d39cc2643ebf4fa1f75c9fafa2a40))
* **web:** add interactive demo website ([4ae1f4c](https://github.com/mcthesw/easy-nats/commit/4ae1f4c32ddfc139eb67efb03b52cdad9531169a))
* **web:** build minimal public website ([60e1ac4](https://github.com/mcthesw/easy-nats/commit/60e1ac47fa5034c06a807aded6805b57698d2f75))


### Bug Fixes

* **ci:** generate WASM before website checks ([0876673](https://github.com/mcthesw/easy-nats/commit/087667343fdfbbb2fc3fe6f044879c494dd0e236))
* resize macOS app icon ([22e30a9](https://github.com/mcthesw/easy-nats/commit/22e30a96e95c7e11d0c894c73cc18911c43f35db))

## [0.1.30](https://github.com/mcthesw/easy-nats/compare/v0.1.29...v0.1.30) (2026-07-22)


### Features

* **theme:** add theme-aware payload syntax highlighting ([eb68c77](https://github.com/mcthesw/easy-nats/commit/eb68c77932ba743f8b31eba8fb435e3d1bb0af1c))


### Bug Fixes

* **homebrew:** use supported macOS dependency syntax ([de64374](https://github.com/mcthesw/easy-nats/commit/de643742490595a429c7516a19e5b301944b3c22))
* **ui:** expand subscriber payload preview ([29d7fe2](https://github.com/mcthesw/easy-nats/commit/29d7fe2990e1cee10b9ad00fbb869f3d7cee5d76))

## [0.1.29](https://github.com/mcthesw/easy-nats/compare/v0.1.28...v0.1.29) (2026-07-09)


### Features

* **settings:** hide backing streams in sidebar by default ([573f852](https://github.com/mcthesw/easy-nats/commit/573f852e92a2aa02ed83f04477d908ce55cd77cf))

## [0.1.28](https://github.com/mcthesw/easy-nats/compare/v0.1.27...v0.1.28) (2026-07-08)


### Features

* **search:** show value payloads in workspace previews ([dea92e2](https://github.com/mcthesw/easy-nats/commit/dea92e22b6067b432930bb625a9c3c71633ee5ca))


### Bug Fixes

* **kv:** handle missing entry cleanup ([cee5f1a](https://github.com/mcthesw/easy-nats/commit/cee5f1a45b1c4278e78baa0c7db37ae975693add))

## [0.1.27](https://github.com/mcthesw/easy-nats/compare/v0.1.26...v0.1.27) (2026-06-28)


### Features

* **app:** route pubsub request reply state ([affebee](https://github.com/mcthesw/easy-nats/commit/affebee2dd5ba2c55548d8cb5e63782fd9f761e7))
* **backend:** add request reply operations ([cd1da2c](https://github.com/mcthesw/easy-nats/commit/cd1da2c3f93b9a00d2c8c34960f2ebbd0dfc7714))
* **tabs:** add request reply workflows ([72fa638](https://github.com/mcthesw/easy-nats/commit/72fa638916ab32ded00f37f672f49df1098f0ce9))

## [0.1.26](https://github.com/mcthesw/easy-nats/compare/v0.1.25...v0.1.26) (2026-06-27)


### Features

* add MsgPack payload display ([db1d66f](https://github.com/mcthesw/easy-nats/commit/db1d66f924eb2ed77ad5b7840a2154d259767e52))
* add MsgPack payload input mode ([3762861](https://github.com/mcthesw/easy-nats/commit/3762861ca6b6272ce572d1667a134f86d521113e))

## [0.1.25](https://github.com/mcthesw/easy-nats/compare/v0.1.24...v0.1.25) (2026-06-19)


### Bug Fixes

* **connection:** close bound tabs on disconnect ([2eec10e](https://github.com/mcthesw/easy-nats/commit/2eec10e972903082a275eaac63e807d4bb86bcdb))

## [0.1.24](https://github.com/mcthesw/easy-nats/compare/v0.1.23...v0.1.24) (2026-06-16)


### Features

* **search:** add formatted result previews ([4f6d720](https://github.com/mcthesw/easy-nats/commit/4f6d720c0ded721b07d83b562b45e5e6eb9590d8))

## [0.1.23](https://github.com/mcthesw/easy-nats/compare/v0.1.22...v0.1.23) (2026-05-28)


### Bug Fixes

* **theme:** persist egui theme preference ([2b0c57a](https://github.com/mcthesw/easy-nats/commit/2b0c57a2a607457c73c06e766df5cc0e64e13b2a))

## [0.1.22](https://github.com/mcthesw/easy-nats/compare/v0.1.21...v0.1.22) (2026-05-27)


### Bug Fixes

* **kv:** expand history panel height ([e82b14b](https://github.com/mcthesw/easy-nats/commit/e82b14b6bed9c1fbbe77d13369be4f89add5225e))

## [0.1.21](https://github.com/mcthesw/easy-nats/compare/v0.1.20...v0.1.21) (2026-05-19)


### Bug Fixes

* **kv:** keep key list scrollbar aligned ([c4954b9](https://github.com/mcthesw/easy-nats/commit/c4954b90d601eb845d511e4cd9e33aabeb409365))

## [0.1.20](https://github.com/mcthesw/easy-nats/compare/v0.1.19...v0.1.20) (2026-05-16)


### Performance Improvements

* **search:** optimize in-memory search refresh ([f15d3c5](https://github.com/mcthesw/easy-nats/commit/f15d3c5bebd4657e2bb2526f729523fc0719541f))


### Documentation

* add local sandbox quick start ([4922542](https://github.com/mcthesw/easy-nats/commit/492254208d25b415303c72ee101565e6ee114de5))

## [0.1.19](https://github.com/mcthesw/easy-nats/compare/v0.1.18...v0.1.19) (2026-05-08)


### Bug Fixes

* **search:** prevent source chips from deforming ([075ff2d](https://github.com/mcthesw/easy-nats/commit/075ff2dcf520432fa178fe57a3bdab24d867c2dc))

## [0.1.18](https://github.com/mcthesw/easy-nats/compare/v0.1.17...v0.1.18) (2026-05-07)


### Features

* **monitoring:** add client status tab ([7990bf0](https://github.com/mcthesw/easy-nats/commit/7990bf0b5510b629294089444d268a5e665fb0e3))

## [0.1.17](https://github.com/mcthesw/easy-nats/compare/v0.1.16...v0.1.17) (2026-05-06)


### Bug Fixes

* **consumer:** guard workqueue preview limitation ([87b2a1b](https://github.com/mcthesw/easy-nats/commit/87b2a1b3bc10806fd4363db4127abe608cd8147e))


### Performance Improvements

* **kv:** virtualize large key lists ([2f90056](https://github.com/mcthesw/easy-nats/commit/2f90056c4934d83897cfbc7b04143a1b3c065f93))

## [0.1.16](https://github.com/mcthesw/easy-nats/compare/v0.1.15...v0.1.16) (2026-05-01)


### Bug Fixes

* **ci:** resolve release tag input first ([de7b289](https://github.com/mcthesw/easy-nats/commit/de7b28973a647cad4b87e3ffe085244d8dbf1981))

## [0.1.15](https://github.com/mcthesw/easy-nats/compare/v0.1.14...v0.1.15) (2026-05-01)


### Features

* **consumer:** support all deliver policies ([03a0534](https://github.com/mcthesw/easy-nats/commit/03a0534ad2c1a10aa6147ea72c1f1ef420624f6e))
* **kv:** show stored values and current key count ([cd0b01a](https://github.com/mcthesw/easy-nats/commit/cd0b01aac179bf8118ec1b6160395f86c5dae50e))


### Code Refactoring

* **app:** wire typed backend domain state ([2628462](https://github.com/mcthesw/easy-nats/commit/2628462d5c68a98c7c1cdb6f98dca168cd202b83))
* **backend:** type domain commands and events ([cf16cb7](https://github.com/mcthesw/easy-nats/commit/cf16cb78650615932102f8c056f1db6de22dd8c9))
* **object-store:** use typed bucket and object models ([1f191c3](https://github.com/mcthesw/easy-nats/commit/1f191c3d61bdbe82c403f12359e5716123cbfa5b))
* **server-info:** use typed info snapshots ([f430d92](https://github.com/mcthesw/easy-nats/commit/f430d921d13ac26195e6be3fb43b9ba0a9eb5141))
* **stream:** use typed stream and consumer models ([9cfaf50](https://github.com/mcthesw/easy-nats/commit/9cfaf5088e04c06db19de79f8d9a209143152413))


### Documentation

* **openspec:** propose typed backend domain models ([2bb32fc](https://github.com/mcthesw/easy-nats/commit/2bb32fc4d5549aeba501d828ca26e15575647963))

## [0.1.14](https://github.com/mcthesw/easy-nats/compare/v0.1.13...v0.1.14) (2026-04-29)


### Features

* add message schema management ([72b395d](https://github.com/mcthesw/easy-nats/commit/72b395d863c689cb0edf5b7af7260be98db95abd))
* add schema payload templates ([fb2d566](https://github.com/mcthesw/easy-nats/commit/fb2d5664fe1ae8bdca292098c89435294a7421a2))
* apply schemas to stream and kv payloads ([22d93d1](https://github.com/mcthesw/easy-nats/commit/22d93d1e52c876ccf5ac434c015ea08693bd7ae2))
* retain longer metrics history ([df16596](https://github.com/mcthesw/easy-nats/commit/df16596c55f8855cf517ccf8fa30e935761ffe6c))


### Code Refactoring

* split message schema modules ([e168929](https://github.com/mcthesw/easy-nats/commit/e168929958f4be8361e5ce528ca1fb149f6a3ed8))

## [0.1.13](https://github.com/mcthesw/easy-nats/compare/v0.1.12...v0.1.13) (2026-04-26)


### Features

* **search:** add in-memory search workspace ([1edbc89](https://github.com/mcthesw/easy-nats/commit/1edbc892eaa11c84628b39d0ce91747ac2470989))


### Bug Fixes

* **release:** avoid cargo-workspace version inheritance ([7ef7143](https://github.com/mcthesw/easy-nats/commit/7ef7143e027481d40a39762639bb541d43c7bfe5))
* **release:** sync Cargo lockfile updates ([2510a58](https://github.com/mcthesw/easy-nats/commit/2510a58c2771c93dd1ed6393af621faf99af3c44))
* **search:** harden KV scans and sync release metadata ([630437a](https://github.com/mcthesw/easy-nats/commit/630437a5a031972a0db7651ee1ff8a5c297cfcca))

## [0.1.12](https://github.com/mcthesw/easy-nats/compare/v0.1.11...v0.1.12) (2026-04-25)


### Features

* **search:** add scoped tab search ([e3bb8ee](https://github.com/mcthesw/easy-nats/commit/e3bb8ee453f76fd9e9481da144f5f39452704116))


### Documentation

* **roadmap:** focus on planned work ([22485db](https://github.com/mcthesw/easy-nats/commit/22485db9736575f4f1c2900c7bb5864a139410d9))

## [0.1.11](https://github.com/mcthesw/easy-nats/compare/v0.1.10...v0.1.11) (2026-04-23)


### Features

* **pubsub:** restore clickable topic history and tab cycling ([8d4535d](https://github.com/mcthesw/easy-nats/commit/8d4535d32c8a2fcb22f44f5e5da517f13d8d65d6))


### Documentation

* refresh README ([43383f7](https://github.com/mcthesw/easy-nats/commit/43383f7f773c93961fad5c5baede6576f5d0d2e8))

## [0.1.10](https://github.com/mcthesw/easy-nats/compare/v0.1.9...v0.1.10) (2026-04-23)


### Features

* add configurable Pub/Sub tab reuse behavior ([f31ca9e](https://github.com/mcthesw/easy-nats/commit/f31ca9eee42e36d5f42f2a5d47e4a4cd51fec8e8))
* **metrics:** add connection-scoped monitoring dashboard ([38747b8](https://github.com/mcthesw/easy-nats/commit/38747b8fec9f4ee71069602da9025ebbe3253068))

## [0.1.9](https://github.com/mcthesw/easy-nats/compare/v0.1.8...v0.1.9) (2026-04-20)


### Features

* **flatpak:** enhance Flathub metadata ([e781b5d](https://github.com/mcthesw/easy-nats/commit/e781b5d5425af4e725e8650b8448f7c7d6266a6f))

## [0.1.8](https://github.com/mcthesw/easy-nats/compare/v0.1.7...v0.1.8) (2026-04-18)


### Performance Improvements

* update eframe dependencies for smaller size ([76f554e](https://github.com/mcthesw/easy-nats/commit/76f554e6a7f0da51a3f211c0824e03f05696acb8))

## [0.1.7](https://github.com/mcthesw/easy-nats/compare/v0.1.6...v0.1.7) (2026-04-18)


### Build System

* add app icon pipeline and cargo workspace release support ([e505e6f](https://github.com/mcthesw/easy-nats/commit/e505e6ffef9ec60e60937f15062a838a480b4afd))

## [0.1.6](https://github.com/mcthesw/easy-nats/compare/v0.1.5...v0.1.6) (2026-04-18)


### Bug Fixes

* **publisher:** route request events by backend id ([6b5e1f4](https://github.com/mcthesw/easy-nats/commit/6b5e1f4a8095673a5de80615947385df26375756))


### Performance Improvements

* **app:** lower idle footprint ([e12afca](https://github.com/mcthesw/easy-nats/commit/e12afca1c62e459d6a85504b8a847439a10e08f7))
* **backend:** batch subscriber events ([30966a4](https://github.com/mcthesw/easy-nats/commit/30966a489a669d50f3d64f75cf36a4c4c3dd6c92))
* **subscriber:** cache visible rows ([f184783](https://github.com/mcthesw/easy-nats/commit/f18478303b5dcda2e909dc3abfb31533944d9ec6))


### Code Refactoring

* **backend:** replace operation strings with enum ([7f572ce](https://github.com/mcthesw/easy-nats/commit/7f572cec538a093b049610431fd30223e0a29497))

## [0.1.5](https://github.com/mcthesw/easy-nats/compare/v0.1.4...v0.1.5) (2026-04-17)


### Features

* **theme:** implement Catppuccin theme support and refactor theme handling ([b7d0335](https://github.com/mcthesw/easy-nats/commit/b7d0335d7c928b7ae1ef8c65464ffa5094928007))

## [0.1.4](https://github.com/mcthesw/easy-nats/compare/v0.1.3...v0.1.4) (2026-04-17)


### Bug Fixes

* **release:** align release-please PR title parsing ([#6](https://github.com/mcthesw/easy-nats/issues/6)) ([80d8ae0](https://github.com/mcthesw/easy-nats/commit/80d8ae03a43e1b2ad98e52db12d7f35810147b39))
* **release:** avoid grouped release PR parsing ([#7](https://github.com/mcthesw/easy-nats/issues/7)) ([848427b](https://github.com/mcthesw/easy-nats/commit/848427bbed56b48a1227ebd5802ec56c1f8633b4))
* **release:** simplify single-package title parsing ([#9](https://github.com/mcthesw/easy-nats/issues/9)) ([3cd88f2](https://github.com/mcthesw/easy-nats/commit/3cd88f21f2f980ef47ec5532d9ff9f454ccc98b1))

## [0.1.3](https://github.com/mcthesw/easy-nats/compare/v0.1.2...v0.1.3) (2026-04-16)


### Bug Fixes

* **release:** handle spaces in packaged asset names ([04a1726](https://github.com/mcthesw/easy-nats/commit/04a17262154d81c00f67393817574ce09ee65bca))
* **release:** normalize macOS asset names before upload ([c06a161](https://github.com/mcthesw/easy-nats/commit/c06a1612d2843f07d9f52a32d18d895d1c1ead9d))

## [0.1.2](https://github.com/mcthesw/easy-nats/compare/v0.1.1...v0.1.2) (2026-04-16)


### Bug Fixes

* **release:** stabilize self-hosted publishing ([36e4039](https://github.com/mcthesw/easy-nats/commit/36e40393f35aaa0b87d6f2f46ab957d46a06714f))

## [0.1.1](https://github.com/mcthesw/easy-nats/compare/v0.1.0...v0.1.1) (2026-04-16)


### Features

* add Docker Compose setup and seed scripts for NATS server and traffic generation ([04a63d6](https://github.com/mcthesw/easy-nats/commit/04a63d644bcae665caad59f050ebf6f101223e3e))
* add main menu image to README and update layout ([0191f1b](https://github.com/mcthesw/easy-nats/commit/0191f1b9fdba900de1993090a97b06bf5242ca77))
* add object store backend ([6a940f7](https://github.com/mcthesw/easy-nats/commit/6a940f72d250cf8ba023b4d99f949d66e4300f4e))
* add protobuf support and object store UI ([ce255af](https://github.com/mcthesw/easy-nats/commit/ce255af3122116d3034dd49e9b174c56aeb31b73))
* add settings tab and filtered log viewer ([9d4b474](https://github.com/mcthesw/easy-nats/commit/9d4b4743500289b175fd60b6b69714994400dd6b))
* **app:** add auto-refresh for stream consumers and KV keys ([778062e](https://github.com/mcthesw/easy-nats/commit/778062e2153927c4ecdbe6daa742a6656dccbdc2))
* **app:** add WorkQueue retention warning in stream browser ([f95b3d0](https://github.com/mcthesw/easy-nats/commit/f95b3d0c94a78b15708198e646e10c8bd7bf1d00))
* **app:** enhance Welcome tab with centered layout and quick actions ([21ba11a](https://github.com/mcthesw/easy-nats/commit/21ba11a39dede1bb6880019d3558c8fed71d7390))
* bundle Inter and LXGW UI fonts ([dbb9ff2](https://github.com/mcthesw/easy-nats/commit/dbb9ff2ee14915194c0e0a92227f4fbe618bd328))
* centralize all UI strings and add project README ([54193e9](https://github.com/mcthesw/easy-nats/commit/54193e91b77014f0dafcbb5cc4916bf0377f7155))
* **consumer:** add pull consumer message peek ([5fc4bdf](https://github.com/mcthesw/easy-nats/commit/5fc4bdffd95604502642caaa75ac9a7b293000c9))
* **editor:** add consumer and KV bucket config editing ([ccc4dea](https://github.com/mcthesw/easy-nats/commit/ccc4dead927a0be6a690f96095eedd3e0c05f3fe))
* **editor:** add Format JSON button to publisher and KV value editor ([72c5329](https://github.com/mcthesw/easy-nats/commit/72c53295c43537b21a3d6c5a5ed015151968d26a))
* **i18n:** add YAML-based i18n system with en/zh support ([7c94a79](https://github.com/mcthesw/easy-nats/commit/7c94a7925b014e46374b2ec47c8409b2f0ceb374))
* implement async runtime bridge with Tokio worker ([b374e20](https://github.com/mcthesw/easy-nats/commit/b374e20a501be818a6facd45eab4ecc8bfd1ece6))
* implement connection management with auth, persistence, and UI ([fa0735c](https://github.com/mcthesw/easy-nats/commit/fa0735c52a77562e9653ed5b63445a043d5f2e1d))
* implement core NATS publisher with request-reply, headers, timeout, and publisher tab UI ([91b34cc](https://github.com/mcthesw/easy-nats/commit/91b34cc827917983416581ee69d1cf0273b87767))
* implement core NATS subscriber with subscription management, ring buffer, and subscriber tab UI ([4ac7b2b](https://github.com/mcthesw/easy-nats/commit/4ac7b2b1a5881eb5f799aa07357d7fbf95f84521))
* implement JetStream stream management with CRUD, message browsing, and stream tab UI ([3687efa](https://github.com/mcthesw/easy-nats/commit/3687efaa0a186913a29c19dd51eaa8ae27f88639))
* implement message formatting with auto-detect, JSON syntax highlighting, hex dump, and Base64 ([09983c2](https://github.com/mcthesw/easy-nats/commit/09983c28ad3b8e9d5c7208d1a14b7ee689319e66))
* implement workspace UI shell with egui_dock, theme toggle, toast notifications, and resource tree ([ea99133](https://github.com/mcthesw/easy-nats/commit/ea99133c6f551b1868e5a0f6c38c3679d4b07f2f))
* **kv:** redesign KV browser with horizontal split and detail/history toggle ([75de7ef](https://github.com/mcthesw/easy-nats/commit/75de7ef9c2f4377810cfe68877adeed730086212))
* **layout:** resizable message-list/detail split in subscriber and stream tabs ([479a88a](https://github.com/mcthesw/easy-nats/commit/479a88a8ccfb4f976f51700da649e09e865021b1))
* restore emoji fallbacks and toast controls ([098abc4](https://github.com/mcthesw/easy-nats/commit/098abc4fdafe1806ae3916a7be9e6218762751ac))
* scaffold Cargo workspace with app and nats-backend crates ([6534229](https://github.com/mcthesw/easy-nats/commit/65342293f8f584ffa8663edf3ca7da0799a11530))
* **server-info:** add server & JetStream account info panel ([46f1f21](https://github.com/mcthesw/easy-nats/commit/46f1f216540eec688adb63f95301669036817786))
* **stream:** add message timestamps and time-based filtering ([feab95c](https://github.com/mcthesw/easy-nats/commit/feab95c2b3eba7acbfeaca8429c1c39cf0d43202))
* streamline settings and tab focusing ([8e6df2d](https://github.com/mcthesw/easy-nats/commit/8e6df2db01fb99c34a502d9a84dc0645edade21f))
* **subscriber:** support multi-topic subscriptions with subject filter ([eb30ddb](https://github.com/mcthesw/easy-nats/commit/eb30ddb31a3217da55b1d5db1d02011e82dcdde6))
* support TLS First mode for connections ([ee088b1](https://github.com/mcthesw/easy-nats/commit/ee088b1f44d8672b6a4e2737917dc6de56645748))
* **tabs:** add context menu with Close Others, Close All, Close to Right ([0b1bcb4](https://github.com/mcthesw/easy-nats/commit/0b1bcb466dd652df13a4ce6e6d6302962154df37))
* **tabs:** allow multiple publisher/subscriber instances per connection ([46d42df](https://github.com/mcthesw/easy-nats/commit/46d42df7af0572021b01556a957e87d4e928023f))
* unify stream subscriber and kv browsers ([632cce1](https://github.com/mcthesw/easy-nats/commit/632cce1f8d1850c75d438080b651e33e120ca634))


### Bug Fixes

* **ci:** remove cargo-workspace plugin and fix collapsible_match lint ([fcdf7a1](https://github.com/mcthesw/easy-nats/commit/fcdf7a1b1ea7820a83def3ad6402d90863f189e3))
* **gui:** route tracing output to log file when no terminal is attached ([ce962bf](https://github.com/mcthesw/easy-nats/commit/ce962bf53888bdf491b8f24bb744e05d57829c77))
* isolate splitter state per tab ([b052197](https://github.com/mcthesw/easy-nats/commit/b0521970a93032237a1a97f406852b8d2434e04a))
* keep connection toasts visible and unique ([973598b](https://github.com/mcthesw/easy-nats/commit/973598b0c2b4d358a05cfcf0f63b09bf5c1ef0e9))
* multi-bugfix batch  TabGuard RAII, KV cancellation, scrollbar, topic history ([05e0f82](https://github.com/mcthesw/easy-nats/commit/05e0f82861254c95b529add946460379dfcbaa87))
* **object-store:** stream download directly to disk ([135718d](https://github.com/mcthesw/easy-nats/commit/135718db3be00c561331777d31d6c6d2a509c60d))
* patch egui_dock undock ids via fork ([f4e870e](https://github.com/mcthesw/easy-nats/commit/f4e870e02f7b6403e6b4883cfdd81ed2c1b9b327))
* **release:** use simple release-type for workspace compatibility ([6e53ccb](https://github.com/mcthesw/easy-nats/commit/6e53ccbaf239de8d08c25429d194c49db20d7eee))
* surface Tokio runtime startup failure on calling thread ([f06870f](https://github.com/mcthesw/easy-nats/commit/f06870fbec478166ef40eb1eb7f6b74547b7e88c))
* window defaults, theme toggle persistence, and icon rendering ([12f8335](https://github.com/mcthesw/easy-nats/commit/12f83357fb03c1919fdb15a53505578cba082629))


### Code Refactoring

* modularize app shell, tabs, and backend worker into domain-based modules ([edd5d53](https://github.com/mcthesw/easy-nats/commit/edd5d531dcae42c4db1f0ce400923e5e298443fe))
* overhaul logging, subscriber isolation, connection state, and UI ([1546eff](https://github.com/mcthesw/easy-nats/commit/1546effea505d5ab42f3805e226135159c4a9165))
* **paths:** add platform-paths helper with legacy migration ([8372442](https://github.com/mcthesw/easy-nats/commit/837244259b41d6f0d56c8f11412062a120d568bd))
* streamline connection actions in sidebar ([8b500bb](https://github.com/mcthesw/easy-nats/commit/8b500bb43183f1b761cc76185d75930a7198788d))


### Documentation

* add future groundwork placeholders and architecture notes ([9669c5c](https://github.com/mcthesw/easy-nats/commit/9669c5cd37d02e03d7ef9281844d9820bde22e92))

## Changelog
