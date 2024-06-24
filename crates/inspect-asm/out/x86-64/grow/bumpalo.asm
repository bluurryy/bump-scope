inspect_asm::grow::bumpalo:
	push r15
	push r14
	push rbx
	mov rbx, r9
	cmp rdx, r8
	jb .LBB0_0
	mov rax, qword ptr [rdi + 16]
	cmp qword ptr [rax + 32], rsi
	je .LBB0_3
.LBB0_0:
	mov rax, qword ptr [rdi + 16]
	mov r14, qword ptr [rax + 32]
	sub r14, rbx
	jb .LBB0_4
	lea rdx, [r8 - 1]
	not rdx
	and r14, rdx
	cmp r14, qword ptr [rax]
	jb .LBB0_4
	mov qword ptr [rax + 32], r14
	test r14, r14
	je .LBB0_4
.LBB0_1:
	mov rdi, r14
	mov rdx, rcx
	call qword ptr [rip + memcpy@GOTPCREL]
.LBB0_2:
	mov rax, r14
	mov rdx, rbx
	pop rbx
	pop r14
	pop r15
	ret
.LBB0_3:
	lea r9, [rdx - 1]
	mov r10, rdx
	xor r10, r9
	xor r14d, r14d
	cmp r10, r9
	jbe .LBB0_2
	mov r10, rbx
	sub r10, rcx
	movabs r11, -9223372036854775808
	sub r11, rdx
	cmp r10, r11
	ja .LBB0_2
	cmp r10, rsi
	ja .LBB0_0
	mov rdx, rsi
	sub rdx, r10
	not r9
	mov r14, r9
	and r14, rdx
	cmp r14, qword ptr [rax]
	jb .LBB0_0
	mov qword ptr [rax + 32], r14
	test r14, r14
	je .LBB0_0
	mov rdi, r14
	mov rdx, rcx
	call qword ptr [rip + memmove@GOTPCREL]
	jmp .LBB0_2
.LBB0_4:
	mov r14, rsi
	mov rsi, r8
	mov rdx, rbx
	mov r15, rcx
	call qword ptr [rip + bumpalo::Bump::alloc_layout_slow@GOTPCREL]
	mov rsi, r14
	mov rcx, r15
	mov r14, rax
	test rax, rax
	jne .LBB0_1
	xor r14d, r14d
	jmp .LBB0_2
