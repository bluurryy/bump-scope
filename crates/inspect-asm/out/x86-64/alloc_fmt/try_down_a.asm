inspect_asm::alloc_fmt::try_down_a:
	push r14
	push rbx
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
	movups xmm0, xmmword ptr [rip + .L__unnamed_1]
	movaps xmmword ptr [rsp], xmm0
	mov qword ptr [rsp + 16], 0
	mov qword ptr [rsp + 24], rdi
	lea rsi, [rip + .L__unnamed_2]
	mov rdi, rsp
	lea rdx, [rsp + 72]
	call qword ptr [rip + core::fmt::write@GOTPCREL]
	test al, al
	je .LBB0_0
	xor eax, eax
	add rsp, 120
	pop rbx
	pop r14
	ret
.LBB0_0:
	mov rsi, qword ptr [rsp]
	mov rdx, qword ptr [rsp + 8]
	mov rbx, qword ptr [rsp + 24]
	mov rax, qword ptr [rbx]
	cmp rsi, qword ptr [rax]
	je .LBB0_1
	mov rax, rsi
	add rsp, 120
	pop rbx
	pop r14
	ret
.LBB0_1:
	mov rax, qword ptr [rsp + 16]
	add rax, rsi
	xor edi, edi
	sub rax, rdx
	cmovae rdi, rax
	and rdi, -4
	lea rax, [rdx + rsi]
	mov r14, rdi
	cmp rax, rdi
	jbe .LBB0_2
	call qword ptr [rip + memmove@GOTPCREL]
	jmp .LBB0_3
.LBB0_2:
	call qword ptr [rip + memcpy@GOTPCREL]
.LBB0_3:
	mov rcx, qword ptr [rbx]
	mov rax, r14
	mov qword ptr [rcx], r14
	mov rdx, qword ptr [rsp + 8]
	add rsp, 120
	pop rbx
	pop r14
	ret
