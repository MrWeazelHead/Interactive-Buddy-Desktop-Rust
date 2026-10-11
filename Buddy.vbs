Option Explicit
Dim shell, files, root, executable
Set shell = CreateObject("WScript.Shell")
Set files = CreateObject("Scripting.FileSystemObject")
root = files.GetParentFolderName(WScript.ScriptFullName)
executable = root & "\rewrite\target\debug\interactive-buddy.exe"
shell.CurrentDirectory = root & "\rewrite"
If files.FileExists(executable) Then
    shell.Run Chr(34) & executable & Chr(34), 0, False
Else
    shell.Run Chr(34) & root & "\run.cmd" & Chr(34), 1, False
End If
