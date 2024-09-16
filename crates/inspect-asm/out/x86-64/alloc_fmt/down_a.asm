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
	jne .LBB0_8
	mov rsi, qword ptr [rsp]
	mov rbx, qword ptr [rsp + 8]
	mov rax, qword ptr [rsp + 16]
	mov r12, qword ptr [rsp + 24]
	mov rcx, qword ptr [r12]
	mov rcx, qword ptr [rcx]
	cmp rsi, rcx
	je .LBB0_1
	mov r14, rsi
	cmp r14, rcx
	je .LBB0_4
.LBB0_0:
	mov r15, r14
	jmp .LBB0_7
.LBB0_1:
	add rax, rsi
	xor r14d, r14d
	sub rax, rbx
	cmovae r14, rax
	and r14, -4
	lea rax, [rbx + rsi]
	mov rdi, r14
	mov rdx, rbx
	cmp rax, r14
	jbe .LBB0_2
	call qword ptr [rip + memmove@GOTPCREL]
	jmp .LBB0_3
.LBB0_2:
	call qword ptr [rip + memcpy@GOTPCREL]
.LBB0_3:
	mov rax, qword ptr [r12]
	mov qword ptr [rax], r14
	mov rax, qword ptr [r12]
	mov rcx, qword ptr [rax]
	mov rax, rbx
	cmp r14, rcx
	jne .LBB0_0
.LBB0_4:
	add rax, rcx
	xor edx, edx
	sub rax, rbx
	cmovae rdx, rax
	and rdx, -4
	lea rax, [rbx + rcx]
	mov r15, rdx
	sub r15, rcx
	add r15, r14
	mov rdi, r15
	mov rsi, r14
	cmp rax, rdx
	jbe .LBB0_5
	mov rdx, rbx
	call qword ptr [rip + memmove@GOTPCREL]
	jmp .LBB0_6
.LBB0_5:
	mov rdx, rbx
	call qword ptr [rip + memcpy@GOTPCREL]
.LBB0_6:
	mov rax, qword ptr [r12]
	mov qword ptr [rax], r15
.LBB0_7:
	mov rax, r15
	mov rdx, rbx
	add rsp, 120
	pop rbx
	pop r12
	pop r14
	pop r15
	ret
.LBB0_8:
	call qword ptr [rip + bump_scope::private::format_trait_error@GOTPCREL]
	ud2
	mov rcx, qword ptr [rsp]
	mov rdx, qword ptr [rsp + 24]
	mov rdx, qword ptr [rdx]
	cmp rcx, qword ptr [rdx]
	jne .LBB0_9
	add rcx, qword ptr [rsp + 16]
	and rcx, -4
	mov qword ptr [rdx], rcx
.LBB0_9:
	mov rdi, rax
	call _Unwind_Resume@PLT
