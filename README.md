# proofMeWrong

AI agents write software **and** a proof; a deterministic gate checks the proof
against the code. The first property: no secrets in logs.

- [`INTENT.md`](INTENT.md) — intent document (WHY / WHAT / HOW, all decisions)
- [`requirements/auth-service.md`](requirements/auth-service.md) — sample application requirements
- [`properties/no-secrets-in-logs/`](properties/no-secrets-in-logs/) — property package v0.1
- [`.devcontainer/`](.devcontainer/) — all tools in the pinned versions

Quick start (inside the DevContainer):

```
cd properties/no-secrets-in-logs
./run-pipeline.sh doctor
./run-pipeline.sh run --runs 1
```
