Option Explicit
Dim shell, files, root, executable
Set shell = CreateObject("WScript.Shell")
Set files = CreateObject("Scripting.FileSystemObject")
root = files.GetParentFolderName(WScript.ScriptFullName)
executable = root & "\rewrite\target\debug\interactive-buddy.exe"
shell.CurrentDirectory = root & "\rewrite"
If files.FileExists(executable) Then
    shell.Run Chr(34) & executable & Chr(34) & " --playtest", 0, False
Else
    MsgBox "Build Buddy first: cd rewrite, then cargo build --locked.", 48, "Buddy Playtest"
End If
