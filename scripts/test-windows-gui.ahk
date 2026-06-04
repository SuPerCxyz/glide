; test-windows-gui.ahk — Windows GUI automation test for Glide
; Run with: AutoHotkey.exe test-windows-gui.ahk
; Requires: Glide client running and connected to server

#NoEnv
#SingleInstance Force
SetWorkingDir %A_ScriptDir%

ServerUrl := "http://aicode.soocoo.xyz:8080"
TestPassed := 0
TestFailed := 0

Log(msg) {
    FormatTime, timestamp,, yyyy-MM-dd HH:mm:ss
    FileAppend, [%timestamp%] %msg%`n, glide-test.log
}

Check(name, condition) {
    global TestPassed, TestFailed
    if (condition) {
        Log("  PASS: " . name)
        TestPassed++
    } else {
        Log("  FAIL: " . name)
        TestFailed++
    }
}

; === Test 1: Notepad clipboard test ===
Log("=== Test 1: Notepad Clipboard ===")
Run, notepad.exe,,, notepad_pid
WinWait, Untitled - Notepad,, 5
if ErrorLevel {
    Log("  FAIL: Notepad did not open")
    ExitApp 1
}
WinActivate

; Type test text
SendInput, Glide clipboard sync test from Windows{Enter}
Sleep, 500
SendInput, 中文测试 🎉{Enter}
Sleep, 500

; Select all and copy
SendInput, ^a
Sleep, 200
SendInput, ^c
Sleep, 500

; Verify clipboard contains text
clipboardContent := Clipboard
Check("Notepad text copied", InStr(clipboardContent, "Glide clipboard sync test"))
Check("Chinese text in clipboard", InStr(clipboardContent, "中文测试"))

; Close Notepad
WinClose
WinWaitActive, Notepad,, 2
if !ErrorLevel
    SendInput, {Tab}{Enter}

; === Test 2: Browser clipboard test ===
Log("=== Test 2: Browser Clipboard ===")
Clipboard := ""
Sleep, 200

; Write test content to clipboard
Clipboard := "Browser clipboard test 浏览器测试"
Sleep, 200
Check("Clipboard write from script", Clipboard = "Browser clipboard test 浏览器测试")

; === Test 3: Large text ===
Log("=== Test 3: Large Text ===")
largeText := ""
Loop, 10000 {
    largeText .= "x"
}
Clipboard := largeText
Sleep, 200
Check("Large text (10KB) clipboard", StrLen(Clipboard) = 10000)

; === Test 4: Rapid clipboard changes ===
Log("=== Test 4: Rapid Clipboard ===")
Loop, 20 {
    Clipboard := "rapid-test-" . A_Index
    Sleep, 50
}
Check("Rapid clipboard changes", InStr(Clipboard, "rapid-test-"))

; === Summary ===
Log("")
Log("=== RESULTS ===")
Log("Passed: " . TestPassed)
Log("Failed: " . TestFailed)

if (TestFailed > 0) {
    Log("OVERALL: FAILED")
    ExitApp 1
} else {
    Log("OVERALL: PASSED")
    ExitApp 0
}
