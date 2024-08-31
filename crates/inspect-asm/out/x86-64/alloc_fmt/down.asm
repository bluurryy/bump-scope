inspect_asm::alloc_fmt::down:
	push r15
	push r14
	push rbx
	sub rsp, 112
	mov qword ptr [rsp + 32], rsi
	mov qword ptr [rsp + 40], rdx
	lea rax, [rsp + 32]
	mov qword ptr [rsp + 48], rax
	lea rax, [rip + <&T as core::fmt::Display>::fmt]
	mov qword ptr [rsp + 56], rax
	lea rax, [rip + .L__unnamed_0]
	mov qword ptr [rsp + 64], rax
	mov qword ptr [rsp + 72], 2
	mov qword ptr [rsp + 96], 0
	lea rax, [rsp + 48]
	mov qword ptr [rsp + 80], rax
	mov qword ptr [rsp + 88], 1
	movups xmm0, xmmword ptr [rip + .L__unnamed_1]
	movaps xmmword ptr [rsp], xmm0
	mov qword ptr [rsp + 16], 0
	mov qword ptr [rsp + 24], rdi
	lea rsi, [rip + .L__unnamed_2]
	mov rdi, rsp
	lea rdx, [rsp + 64]
	call qword ptr [rip + core::fmt::write@GOTPCREL]
	test al, al
	jne .LBB0_4
	mov rsi, qword ptr [rsp]
	mov rbx, qword ptr [rsp + 8]
	mov r15, qword ptr [rsp + 24]
	mov rax, qword ptr [r15]
	cmp rsi, qword ptr [rax]
	je .LBB0_0
	mov r14, rsi
	jmp .LBB0_3
.LBB0_0:
	mov rax, qword ptr [rsp + 16]
	add rax, rsi
	xor r14d, r14d
	sub rax, rbx
	cmovae r14, rax
	lea rax, [rbx + rsi]
	mov rdi, r14
	mov rdx, rbx
	cmp rax, r14
	jbe .LBB0_1
	call qword ptr [rip + memmove@GOTPCREL]
	jmp .LBB0_2
.LBB0_1:
	call qword ptr [rip + memcpy@GOTPCREL]
.LBB0_2:
	mov rax, qword ptr [r15]
	mov qword ptr [rax], r14
.LBB0_3:
	mov rax, r14
	mov rdx, rbx
	add rsp, 112
	pop rbx
	pop r14
	pop r15
	ret
.LBB0_4:
	call qword ptr [rip + bump_scope::private::format_trait_error@GOTPCREL]
