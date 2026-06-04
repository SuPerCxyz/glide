.PHONY: test test-e2e test-network test-gui test-clipboard test-auth test-keyboard test-all

test: test-e2e

test-e2e:
	python3 scripts/test-e2e.py

test-network:
	bash scripts/test-network.sh

test-gui:
	bash scripts/test-gui-linux.sh

test-clipboard:
	bash scripts/test-clipboard-cli.sh

test-auth:
	bash scripts/test-reconnect.sh

test-keyboard:
	bash scripts/test-keyboard-mouse-protocol.sh

test-network-anomaly:
	bash scripts/test-tc-network.sh

test-all:
	@echo "=== Running all tests ==="
	. "$(HOME)/.cargo/env" && cargo test --package glide-core --package glide-server --package glide-desktop 2>&1 | grep "test result"
	@echo ""
	python3 scripts/test-e2e.py 2>&1 | tail -3
	@echo ""
	bash scripts/test-network.sh 2>&1 | tail -3
	@echo ""
	bash scripts/test-clipboard-cli.sh 2>&1 | tail -3
	@echo ""
	bash scripts/test-keyboard-mouse-protocol.sh 2>&1 | tail -3
	@echo ""
	bash scripts/test-reconnect.sh 2>&1 | tail -3
	@echo ""
	bash scripts/test-gui-linux.sh 2>&1 | grep -E "✅|❌|==="
	@echo ""
	bash scripts/test-tc-network.sh 2>&1 | tail -3
