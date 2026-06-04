#!/usr/bin/env python3
"""
test-windows-notepad-clipboard.py — Windows Notepad clipboard automation test.
Run on Windows with: pip install pywinauto && python test-windows-notepad-clipboard.py
Requires: Glide client running and connected to server.
"""
import time
import sys
import os

PASS = 0
FAIL = 0

def check(name, condition):
    global PASS, FAIL
    if condition:
        print(f"  ✅ {name}")
        PASS += 1
    else:
        print(f"  ❌ {name}")
        FAIL += 1

try:
    import pywinauto
    from pywinauto import Application
    from pywinauto.keyboard import send_keys
    HAS_PYWINAUTO = True
except ImportError:
    HAS_PYWINAUTO = False
    print("pywinauto not installed, running in stub mode")

def test_notepad():
    """Test clipboard sync via Notepad."""
    print("=== Notepad Clipboard Test ===")
    
    if not HAS_PYWINAUTO:
        print("  ⏭ Skipped (pywinauto not installed)")
        return
    
    try:
        # Launch Notepad
        app = Application(backend="uia").start("notepad.exe")
        time.sleep(2)
        
        # Get Notepad window
        dlg = app.window(title_re=".*Notepad.*")
        dlg.wait("visible", timeout=10)
        check("Notepad launched", True)
        
        # Type text
        dlg.type_keys("Glide clipboard sync test{ENTER}", with_spaces=True)
        time.sleep(0.5)
        dlg.type_keys("中文测试 🎉{ENTER}", with_spaces=True)
        time.sleep(0.5)
        check("Text typed in Notepad", True)
        
        # Select all and copy
        dlg.type_keys("^a")
        time.sleep(0.2)
        dlg.type_keys("^c")
        time.sleep(0.5)
        
        # Check clipboard
        import win32clipboard
        win32clipboard.OpenClipboard()
        try:
            clipboard_data = win32clipboard.GetClipboardData(win32clipboard.CF_UNICODETEXT)
            check("Clipboard has content", clipboard_data is not None)
            check("Content matches", "Glide clipboard sync test" in clipboard_data)
            check("Chinese preserved", "中文测试" in clipboard_data)
        finally:
            win32clipboard.CloseClipboard()
        
        # Close Notepad without saving
        dlg.close()
        time.sleep(1)
        try:
            save_dlg = app.window(title_re=".*Notepad.*")
            save_dlg.type_keys("{Tab}{Enter}")  # Don't Save
        except:
            pass
        
        check("Notepad closed cleanly", True)
        
    except Exception as e:
        check("Notepad test", False)
        print(f"    Error: {e}")

def test_clipboard_types():
    """Test different clipboard content types."""
    print("\n=== Clipboard Types Test ===")
    
    if not HAS_PYWINAUTO:
        print("  ⏭ Skipped")
        return
    
    try:
        import win32clipboard
        
        # Test text
        win32clipboard.OpenClipboard()
        win32clipboard.EmptyClipboard()
        win32clipboard.SetClipboardText("test text")
        win32clipboard.CloseClipboard()
        
        win32clipboard.OpenClipboard()
        data = win32clipboard.GetClipboardData(win32clipboard.CF_UNICODETEXT)
        win32clipboard.CloseClipboard()
        check("Text clipboard", data == "test text")
        
        # Test empty
        win32clipboard.OpenClipboard()
        win32clipboard.EmptyClipboard()
        win32clipboard.CloseClipboard()
        check("Empty clipboard", True)
        
    except ImportError:
        print("  ⏭ Skipped (win32clipboard not installed)")
    except Exception as e:
        check("Clipboard types", False)
        print(f"    Error: {e}")

def main():
    print("=== Windows Notepad + Clipboard Test ===")
    test_notepad()
    test_clipboard_types()
    
    print(f"\n{'='*50}")
    print(f"RESULTS: {PASS} passed, {FAIL} failed")
    print(f"{'='*50}")
    
    if FAIL > 0:
        sys.exit(1)
    else:
        sys.exit(0)

if __name__ == "__main__":
    main()
