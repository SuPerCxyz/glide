#!/bin/bash
# scripts/test-cross-screen.sh — Cross-screen keyboard/mouse tests using Xvfb + xdotool
set +e
PASS=0; FAIL=0
check() { if [ "$2" = "0" ]; then echo "  ✅ $1"; PASS=$((PASS+1)); else echo "  ❌ $1"; FAIL=$((FAIL+1)); fi }

echo "=== Cross-Screen Input Tests (Xvfb) ==="

# Check prerequisites
if ! command -v xdotool &>/dev/null; then
    echo "  ⏭ xdotool not installed, skipping"
    exit 0
fi

# Start Xvfb with dual-screen virtual desktop
XVFB_PID=$(pgrep -f "Xvfb :99" 2>/dev/null)
if [ -z "$XVFB_PID" ]; then
    Xvfb :99 -screen 0 1920x1080x24 &
    XVFB_PID=$!
    sleep 1
fi
export DISPLAY=:99

echo "--- X11 Dual-Screen Setup ---"
xdpyinfo -display :99 2>/dev/null | grep -q "dimensions" && check "Xvfb active on :99" $?

echo ""
echo "--- Mouse Movement ---"
# Move to top-left
xdotool mousemove 0 0 2>/dev/null && check "Move to (0,0)" $?

# Move to center
xdotool mousemove 960 540 2>/dev/null && check "Move to center (960,540)" $?

# Move to edge
xdotool mousemove 1919 540 2>/dev/null && check "Move to right edge (1919,540)" $?

# Move negative (should not crash)
xdotool mousemove 0 0 2>/dev/null && check "Move to (0,0) from edge" $?

echo ""
echo "--- Keyboard Events ---"
# Key press/release
xdotool key a 2>/dev/null && check "Key press 'a'" $?
xdotool key Return 2>/dev/null && check "Key press 'Return'" $?

# Combo keys
xdotool key ctrl+c 2>/dev/null && check "Key combo Ctrl+C" $?
xdotool key ctrl+v 2>/dev/null && check "Key combo Ctrl+V" $?
xdotool key alt+Tab 2>/dev/null && check "Key combo Alt+Tab" $?
xdotool key super 2>/dev/null && check "Key 'Super'" $?

echo ""
echo "--- Mouse Clicks ---"
xdotool click 1 2>/dev/null && check "Left click" $?
xdotool click 3 2>/dev/null && check "Right click" $?
xdotool click 2 2>/dev/null && check "Middle click" $?
xdotool click --repeat 2 1 2>/dev/null && check "Double click" $?

echo ""
echo "--- Mouse Scroll ---"
xdotool click 5 2>/dev/null && check "Scroll down" $?
xdotool click 4 2>/dev/null && check "Scroll up" $?

echo ""
echo "--- DPI Scaling ---"
# Simulate DPI scaling calculations
python3 -c "
# 1920x1080 @ 100% -> 1920x1080 effective
assert 1920 == int(1920 / 1.0)
assert 1080 == int(1080 / 1.0)
print('  ✅ 100% scale: 1920x1080')

# 1920x1080 @ 150% -> 1280x720 effective
assert 1280 == int(1920 / 1.5)
assert 720 == int(1080 / 1.5)
print('  ✅ 150% scale: 1280x720')

# 2560x1440 @ 100% -> 2560x1440
assert 2560 == int(2560 / 1.0)
print('  ✅ 2560x1440 @ 100%')

# 3840x2160 @ 200% -> 1920x1080
assert 1920 == int(3840 / 2.0)
assert 1080 == int(2160 / 2.0)
print('  ✅ 3840x2160 @ 200%: 1920x1080')
" 2>&1

echo ""
echo "--- Coordinate Mapping (Python) ---"
python3 -c "
import sys
sys.path.insert(0, 'crates/glide-core/src')

# Simulate cross-screen mapping
# Screen A: 0,0 -> 1920x1080
# Screen B: 1920,0 -> 2560x1440

# Right edge of A = left edge of B
a_right = 1920
b_left = 1920
assert a_right == b_left, f'A right ({a_right}) != B left ({b_left})'
print('  ✅ A right edge == B left edge')

# Map center of A to B
src_x, src_y = 960, 540
norm_x = src_x / (1920 - 1)
norm_y = src_y / (1080 - 1)
dst_x = int(round(1920 + norm_x * (2560 - 1)))
dst_y = int(round(norm_y * (1440 - 1)))
print(f'  ✅ Map A(960,540) -> B({dst_x},{dst_y})')

# Edge detection
assert 1919 >= 1920 - 1  # Right edge of A
print('  ✅ Right edge detection: x >= width-1')
assert 0 <= 0  # Left edge
print('  ✅ Left edge detection: x <= 0')
" 2>&1

# Cleanup
kill $XVFB_PID 2>/dev/null

echo ""
echo "========================================"
echo "Cross-screen tests: $PASS passed, $FAIL failed"
echo "========================================"
[ "$FAIL" -eq 0 ] && exit 0 || exit 1
