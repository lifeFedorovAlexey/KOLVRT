.section .rodata.security_image,"a"
.balign 16
.global security_image_start, security_image_end
.macro sec_request op, h=xzr
 add x28,x28,#1
 mov x0, #1
 str x0, [x19]
 mov x0, #\op
 str x0, [x19, #8]
 str \h, [x19, #16]
 stp xzr, xzr, [x19, #24]
 str xzr, [x19, #40]
 mov x0, x19
 svc #0x90
.endm
security_image_start:
 mov x28,#100
 // Trusted bootstrap provides role and local tokens. All calls use production endpoints.
 mov x19, #0x20000000
 add x19, x19, #4096
 cmp x20, #1
 b.eq .Lsec_server
 cmp x20, #3
 b.eq .Lsec_success
 cmp x20, #4
 b.eq .Lsec_revoker
 cmp x20, #5
 b.eq .Lsec_racer
 cmp x20,#6
 b.eq .Lsec_quota_client
 cmp x20,#7
 b.eq .Lsec_quota_peer
 cmp x20,#8
 b.eq .Lsec_wait_server
 cmp x20,#9
 b.eq .Lsec_simple_client
 cmp x20,#10
 b.eq .Lsec_simple_client
 cmp x20,#11
 b.eq .Lsec_wait_server
 cmp x20,#12
 b.eq .Lsec_simple_client
 cmp x20,#13
 b.eq .Lsec_server
 cmp x20,#14
 b.eq .Lsec_simple_client
 // Client: foreign memory snapshot fails before effects.
 mov x0, x24
 svc #0x90
 cmp x0, #1
 b.ne .Lsec_fail
 svc #0x91
 cmp x0, #1
 b.ne .Lsec_fail
 sec_request 6, x21
 cmp x0, #11
 b.ne .Lsec_fail
 // A valid grant with no SEND right denies admission.
 sec_request 4,x22
 cmp x0,#9
 b.ne .Lsec_fail
 // Inject bare references through the existing test-only reference factory.
 // Production authority admission must still deny the valid non-grant Event.
 mov x1,x19
 svc #0x82
 cmp x0,#1
 b.ne .Lsec_fail
 ldr x22,[x19,#64]
 sec_request 4,x22
 cmp x0,#11
 b.ne .Lsec_fail
 // Unknown rights remain denied by the copied reference boundary.
 mov x0,#1
 str x0,[x19]
 str x0,[x19,#8]
 str x22,[x19,#16]
 str x0,[x19,#24]
 mov x0,#128
 str x0,[x19,#32]
 str xzr,[x19,#40]
 mov x0,x19
 svc #0x80
 cmp x0,#9
 b.ne .Lsec_fail
 // Payload principal/scope selectors are forbidden.
 sec_request 4, x21
 cmp x0, #0
 b.ne .Lsec_fail
 mov x0, #1
 str x0, [x19, #24]
 mov x0, x19
 svc #0x90
 cmp x0, #11
 b.ne .Lsec_fail
 // Request/queue capacity is a denial, never an implicit fallback.
 sec_request 4, x21
 cmp x0, #12
 b.ne .Lsec_fail
 // Unauthorized transfer (SEND-only token) and receiver handle budget.
 mov x0, #1
 str x0, [x19]
 mov x0, #3
 str x0, [x19,#8]
 str x23, [x19,#16]
 str x25, [x19,#24]
 str x26, [x19,#32]
 mov x0, #1
 str x0, [x19,#40]
 mov x0, x19
 svc #0x80
 cmp x0, #9
 b.ne .Lsec_fail
 str x21, [x19,#16]
 mov x0, x19
 svc #0x80
 cmp x0, #12
 b.ne .Lsec_fail
 // Revocation denies new work but does not cancel the already-admitted notification.
 sec_request 5, x21
 cbnz x0, .Lsec_fail
 sec_request 4, x21
 cmp x0, #11
 b.ne .Lsec_fail
 mov x0,#1
 svc #0x93
 cmp x20, #2
 b.eq .Lsec_fault
 b .Lsec_success
.Lsec_simple_client:
 sec_request 4,x21
 cbnz x0,.Lsec_fail
 mov x27,x1
 cmp x20,#12
 b.eq .Lsec_wait_receipt
 cmp x20,#14
 b.eq .Lsec_wait_receipt
 cmp x20,#10
 b.eq .Lsec_fault
 b .Lsec_success
.Lsec_wait_receipt:
 svc #0x55
 add x29,x27,#1
 sec_request 10,x29
 cmp x0,#11
 b.ne .Lsec_fail
 sec_request 10,x27
 cmp x20,#14
 b.eq .Lsec_cancelled
 cmp x0,#1
 b.ne .Lsec_fail
 sec_request 10,x27
 cmp x0,#11
 b.ne .Lsec_fail
 // Old service generation cannot bind to a restarted service.
 sec_request 4,x21
 cmp x0,#11
 b.ne .Lsec_fail
 b .Lsec_success
.Lsec_cancelled:
 cmp x0,#2
 b.ne .Lsec_fail
 b .Lsec_success
.Lsec_wait_server:
 svc #0x55
 cmp x20,#11
 b.eq .Lsec_service_fault
 b .Lsec_server
.Lsec_service_fault:
 mov x0,#0
 ldr x0,[x0]
 b .Lsec_fail
.Lsec_quota_client:
 sec_request 4,x21
 cmp x0,#12
 b.ne .Lsec_fail
 mov x0,#1
 svc #0x93
 b .Lsec_success
.Lsec_quota_peer:
 mov x0,#0
 svc #0x93
 cbz x0,.Lsec_quota_peer
 b .Lsec_success
.Lsec_server:
 sec_request 8
 cmp x0, #11
 b.eq .Lsec_server
 cmp x0, #1
 b.ne .Lsec_fail
 cmp x20,#13
 b.eq .Lsec_server_seen
 cbz x1, .Lsec_server
.Lsec_server_seen:
 cmp x2, #1
 b.ne .Lsec_fail
 // A service-private token cannot replace the admitted consumer effect target.
 sec_request 9, x21
 cmp x0, #11
 b.ne .Lsec_fail
 cbz x22, .Lsec_server_finish
 // Delegated SEND token cannot acquire TRANSFER authority.
 mov x0,#1
 str x0,[x19]
 str x0,[x19,#8]
 str x22,[x19,#16]
 str x0,[x19,#24]
 mov x0,#2
 str x0,[x19,#32]
 str xzr,[x19,#40]
 mov x0,x19
 svc #0x80
 cmp x0,#9
 b.ne .Lsec_fail
.Lsec_server_finish:
 sec_request 9
 cbnz x0,.Lsec_fail
 sec_request 9
 cmp x0,#11
 b.ne .Lsec_fail
 b .Lsec_success
.Lsec_racer:
 sec_request 4, x21
 cbnz x0,.Lsec_fail
 mov x0,#1
 svc #0x92
.Lsec_race_loop:
 sec_request 4, x21
 cmp x0,#11
 b.eq .Lsec_success
 cmp x0,#12
 b.ne .Lsec_fail
 mov x0,#2
 svc #0x92
 b .Lsec_race_loop
.Lsec_revoker:
 mov x0,#0
 svc #0x92
 cmp x0,#2
 b.lo .Lsec_revoker
 sec_request 5,x21
 cbnz x0,.Lsec_fail
 b .Lsec_success
.Lsec_fault:
 mov x0,x24
 ldr x0,[x0]
 b .Lsec_fail
.Lsec_success:
 mov x0,#1
 svc #0x4e
.Lsec_fail:
 add x0,x0,x28,lsl #8
 svc #0x4e
 b .Lsec_fail
security_image_end:
