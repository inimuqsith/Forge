# Source Code Package Manager Forge

Modul engine Forge mencakup:
- CLI command dispatcher & options parser.
- Config parser (`forge.conf`).
- Recipe DSL parser & validator.
- Fetcher & SHA256 integrity verifier.
- RAM tmpfs sandbox build runner & CFLAGS injector.
- DESTDIR staging engine.
- Transactional merger, file collision detector, & manifest database manager.
- OpenRC service hook triggers.
- Stage tarball exporter (`forge stage-export`).
