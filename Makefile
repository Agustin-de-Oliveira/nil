.PHONY: help shot clip notes build build-shot build-clip build-notes check clean

help:
	@echo "nil suite:"
	@echo "  make shot        - Inicia nil-shot en modo dev"
	@echo "  make clip        - Inicia nil-clip en modo dev"
	@echo "  make notes       - Inicia nil-notes en modo dev"
	@echo "  make build       - Compila los 3 frontends y binarios release"
	@echo "  make check       - Verifica codigo con cargo check"
	@echo "  make clean       - Limpia target/ y dist/"

shot:
	@(cd frontend/nil-shot && ../../bin/trunk serve) & TRUNK_PID=$$!; \
	trap 'kill $$TRUNK_PID 2>/dev/null' EXIT INT TERM; \
	sleep 1; \
	cargo run -p nil-shot

clip:
	@(cd frontend/nil-clip && ../../bin/trunk serve) & TRUNK_PID=$$!; \
	trap 'kill $$TRUNK_PID 2>/dev/null' EXIT INT TERM; \
	sleep 1; \
	cargo run -p nil-clip

notes:
	@(cd frontend/nil-notes && ../../bin/trunk serve) & TRUNK_PID=$$!; \
	trap 'kill $$TRUNK_PID 2>/dev/null' EXIT INT TERM; \
	sleep 1; \
	cargo run -p nil-notes

build-shot:
	@cd frontend/nil-shot && ../../bin/trunk build
	@cargo build -p nil-shot --release

build-clip:
	@cd frontend/nil-clip && ../../bin/trunk build
	@cargo build -p nil-clip --release

build-notes:
	@cd frontend/nil-notes && ../../bin/trunk build
	@cargo build -p nil-notes --release

build: build-shot build-clip build-notes

check:
	@cargo check --workspace
	@cd frontend/nil-shot && cargo check
	@cd frontend/nil-clip && cargo check
	@cd frontend/nil-notes && cargo check

clean:
	@cargo clean
	@rm -rf frontend/*/dist/
