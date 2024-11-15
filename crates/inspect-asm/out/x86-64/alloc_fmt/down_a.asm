inspect_asm::alloc_fmt::down_a:
	push r15
	push r14
	push r12
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
	jne .LBB0_3
	mov r14, qword ptr [rsp]
	mov rbx, qword ptr [rsp + 8]
	mov r12, qword ptr [rsp + 24]
	mov rax, qword ptr [r12]
	cmp qword ptr [rax], r14
	jne .LBB0_2
	mov rax, qword ptr [rsp + 16]
	add rax, r14
	xor r15d, r15d
	sub rax, rbx
	cmovae r15, rax
	and r15, -4
	lea rax, [rbx + r14]
	mov rdi, r15
	mov rsi, r14
	mov rdx, rbx
	cmp rax, r15
	jbe .LBB0_0
	call qword ptr [rip + memmove@GOTPCREL]
	jmp .LBB0_1
.LBB0_0:
	call qword ptr [rip + memcpy@GOTPCREL]
.LBB0_1:
	mov rax, qword ptr [r12]
	mov qword ptr [rax], r15
	test r15, r15
	cmovne r14, r15
.LBB0_2:
	mov rax, r14
	mov rdx, rbx
	add rsp, 120
	pop rbx
	pop r12
	pop r14
	pop r15
	ret
.LBB0_3:
	call qword ptr [rip + bump_scope::private::format_trait_error@GOTPCREL]
	ud2
	mov rdx, qword ptr [rsp]
	mov rcx, qword ptr [rsp + 24]
	mov rcx, qword ptr [rcx]
	cmp qword ptr [rcx], rdx
	jne .LBB0_4
	mov rsi, qword ptr [rsp + 16]
	add rdx, rsi
	add rdx, 3
	and rdx, -4
	mov qword ptr [rcx], rdx
.LBB0_4:
	mov rdi, rax
	call _Unwind_Resume@PLT
