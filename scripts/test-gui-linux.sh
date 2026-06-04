#!/bin/bash
# scripts/test-gui-linux.sh — Linux GUI headless tests using Xvfb
set +e
echo "=== Linux GUI Headless Tests ==="

# Check Xvfb available
if ! command -v Xvfb &>/dev/null; then
    echo "  ⏭ Xvfb not available, skipping GUI tests"
    exit 0
fi

# Start Xvfb on display :99
Xvfb :99 -screen 0 1920x1080x24 &
XVFB_PID=$!
sleep 1
export DISPLAY=:99

echo "--- X11 Environment ---"
if xdpyinfo -display :99 > /dev/null 2>&1; then
    echo "  ✅ Xvfb display :99 active"
else
    echo "  ❌ Xvfb failed to start"
    kill $XVFB_PID 2>/dev/null
    exit 1
fi

# Test xclip if available
if command -v xclip &>/dev/null; then
    echo ""
    echo "--- xclip Clipboard ---"
    echo "test-x11" | xclip -selection clipboard
    RESULT=$(xclip -selection clipboard -o 2>/dev/null)
    [ "$RESULT" = "test-x11" ] && echo "  ✅ xclip write/read" || echo "  ❌ xclip write/read"
    
    # Chinese text
    echo "你好X11" | xclip -selection clipboard
    RESULT=$(xclip -selection clipboard -o 2>/dev/null)
    [ "$RESULT" = "你好X11" ] && echo "  ✅ xclip Chinese text" || echo "  ❌ xclip Chinese text"
else
    echo "  ⏭ xclip not installed"
fi

# Test xdotool if available
if command -v xdotool &>/dev/null; then
    echo ""
    echo "--- xdotool ---"
    xdotool mousemove 500 300 2>/dev/null && echo "  ✅ xdotool mouse move" || echo "  ❌ xdotool mouse move"
    xdotool key ctrl+c 2>/dev/null && echo "  ✅ xdotool key combo" || echo "  ❌ xdotool key combo"
    xdotool click 1 2>/dev/null && echo "  ✅ xdotool click" || echo "  ❌ xdotool click"
else
    echo "  ⏭ xdotool not installed"
fi

# Cleanup
kill $XVFB_PID 2>/dev/null
echo ""
echo "=== GUI tests done ==="
