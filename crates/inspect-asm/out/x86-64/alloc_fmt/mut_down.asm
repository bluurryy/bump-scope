inspect_asm::alloc_fmt::mut_down:
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
	jne .LBB0_2
	mov rbx, qword ptr [rsp + 16]
	test rbx, rbx
	je .LBB0_0
	mov r15, qword ptr [rsp + 24]
	mov rsi, qword ptr [rsp]
	mov r14, qword ptr [rsp + 8]
	add rbx, rsi
	sub rbx, r14
	mov rdi, rbx
	mov rdx, r14
	call qword ptr [rip + memmove@GOTPCREL]
	mov rax, qword ptr [r15]
	mov qword ptr [rax], rbx
	jmp .LBB0_1
.LBB0_0:
	mov ebx, 1
	xor r14d, r14d
.LBB0_1:
	mov rax, rbx
	mov rdx, r14
	add rsp, 112
	pop rbx
	pop r14
	pop r15
	ret
.LBB0_2:
	call qword ptr [rip + bump_scope::private::format_trait_error@GOTPCREL]
