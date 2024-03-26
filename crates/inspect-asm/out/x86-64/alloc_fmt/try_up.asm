inspect_asm::alloc_fmt::try_up:
	sub rsp, 120
	mov qword ptr [rsp + 40], rsi
	mov qword ptr [rsp + 48], rdx
	lea rax, [rsp + 40]
	mov qword ptr [rsp + 56], rax
	lea rax, [rip + <&T as core::fmt::Display>::fmt]
	mov qword ptr [rsp + 64], rax
	lea rax, [rip + .L__unnamed_0]
	mov qword ptr [rsp + 72], rax
	mov qword ptr [rsp + 80], 2
	mov qword ptr [rsp + 104], 0
	lea rax, [rsp + 56]
	mov qword ptr [rsp + 88], rax
	mov qword ptr [rsp + 96], 1
	mov qword ptr [rsp + 8], 1
	mov qword ptr [rsp + 16], rdi
	xorps xmm0, xmm0
	movups xmmword ptr [rsp + 24], xmm0
	lea rsi, [rip + .L__unnamed_1]
	lea rdi, [rsp + 8]
	lea rdx, [rsp + 72]
	call qword ptr [rip + core::fmt::write@GOTPCREL]
	test al, al
	je .LBB_10
	mov rax, qword ptr [rsp + 24]
	test rax, rax
	je .LBB_5
	mov rcx, qword ptr [rsp + 8]
	mov rdx, qword ptr [rsp + 16]
	add rax, rcx
	mov rdx, qword ptr [rdx]
	cmp rax, qword ptr [rdx]
	je .LBB_4
.LBB_5:
	xor eax, eax
	add rsp, 120
	ret
.LBB_10:
	mov rax, qword ptr [rsp + 8]
	mov rdx, qword ptr [rsp + 32]
	add rsp, 120
	ret
.LBB_4:
	mov qword ptr [rdx], rcx
	xor eax, eax
	add rsp, 120
	ret
	mov rcx, qword ptr [rsp + 24]
	test rcx, rcx
	je .LBB_9
	mov rdx, qword ptr [rsp + 8]
	mov rsi, qword ptr [rsp + 16]
	add rcx, rdx
	mov rsi, qword ptr [rsi]
	cmp rcx, qword ptr [rsi]
	jne .LBB_9
	mov qword ptr [rsi], rdx
.LBB_9:
	mov rdi, rax
	call _Unwind_Resume@PLT