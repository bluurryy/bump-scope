inspect_asm::alloc_fmt::try_down_a:
	sub rsp, 120
	mov qword ptr [rsp + 8], rsi
	mov qword ptr [rsp + 16], rdx
	lea rax, [rsp + 8]
	mov qword ptr [rsp + 24], rax
	lea rax, [rip + <&T as core::fmt::Display>::fmt]
	mov qword ptr [rsp + 32], rax
	lea rax, [rip + .L__unnamed_0]
	mov qword ptr [rsp + 72], rax
	mov qword ptr [rsp + 80], 2
	mov qword ptr [rsp + 104], 0
	lea rax, [rsp + 24]
	mov qword ptr [rsp + 88], rax
	mov qword ptr [rsp + 96], 1
	mov qword ptr [rsp + 40], 1
	xorps xmm0, xmm0
	movups xmmword ptr [rsp + 48], xmm0
	mov qword ptr [rsp + 64], rdi
	lea rsi, [rip + .L__unnamed_1]
	lea rdi, [rsp + 40]
	lea rdx, [rsp + 72]
	call qword ptr [rip + core::fmt::write@GOTPCREL]
	test al, al
	je .LBB_2
	xor eax, eax
	add rsp, 120
	ret
.LBB_2:
	mov rax, qword ptr [rsp + 40]
	mov rdx, qword ptr [rsp + 48]
	add rsp, 120
	ret