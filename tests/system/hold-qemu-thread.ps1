param([int]$QemuPid,[int]$ThreadId,[int]$HoldMs=500)
$ErrorActionPreference='Stop'
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class QemuThreadExperiment {
 [DllImport("kernel32.dll",SetLastError=true)] public static extern IntPtr OpenThread(uint access,bool inherit,uint id);
 [DllImport("kernel32.dll",SetLastError=true)] public static extern uint GetProcessIdOfThread(IntPtr handle);
 [DllImport("kernel32.dll",SetLastError=true)] public static extern uint SuspendThread(IntPtr handle);
 [DllImport("kernel32.dll",SetLastError=true)] public static extern uint ResumeThread(IntPtr handle);
 [DllImport("kernel32.dll")] public static extern bool CloseHandle(IntPtr handle);
}
'@
$threadHandle=[QemuThreadExperiment]::OpenThread(0x0802,$false,$ThreadId)
if($threadHandle -eq [IntPtr]::Zero){throw 'OpenThread failed'}
$held=$false
try {
 if([QemuThreadExperiment]::GetProcessIdOfThread($threadHandle) -ne $QemuPid){throw 'Thread does not belong to owned QEMU process'}
 if([QemuThreadExperiment]::SuspendThread($threadHandle) -eq [uint32]::MaxValue){throw 'SuspendThread failed'}
 $held=$true
 [Console]::WriteLine('HELD'); [Console]::Out.Flush()
 Start-Sleep -Milliseconds $HoldMs
} finally {
 try {
  if($held -and [QemuThreadExperiment]::ResumeThread($threadHandle) -eq [uint32]::MaxValue){throw 'ResumeThread failed'}
 } finally {
  [void][QemuThreadExperiment]::CloseHandle($threadHandle)
 }
}
