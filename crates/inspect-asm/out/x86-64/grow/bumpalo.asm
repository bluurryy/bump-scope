inspect_asm::grow::bumpalo:
	push r15
	push r14
	push rbx
	mov rbx, r9
	cmp rdx, r8
	jb .LBB_1
	mov rax, qword ptr [rdi + 16]
	cmp qword ptr [rax + 32], rsi
	je .LBB_7
.LBB_1:
	mov rax, qword ptr [rdi + 16]
	mov r14, qword ptr [rax + 32]
	sub r14, rbx
	jb .LBB_4
	lea rdx, [r8 - 1]
	not rdx
	and r14, rdx
	cmp r14, qword ptr [rax]
	jb .LBB_4
	mov qword ptr [rax + 32], r14
	test r14, r14
	je .LBB_4
.LBB_13:
	mov rdi, r14
	mov rdx, rcx
	call qword ptr [rip + memcpy@GOTPCREL]
.LBB_14:
	mov rax, r14
	mov rdx, rbx
	pop rbx
	pop r14
	pop r15
	ret
.LBB_7:
	mov r9, rbx
	sub r9, rcx
	lea r10, [rdx - 1]
	mov r11, rdx
	xor r11, r10
	movabs r15, -9223372036854775808
	sub r15, rdx
	xor r14d, r14d
	cmp r15, r9
	cmovb rdx, r14
	cmp r11, r10
	jbe .LBB_14
	test rdx, rdx
	je .LBB_14
	cmp r9, rsi
	ja .LBB_1
	mov r10, rsi
	sub r10, r9
	neg rdx
	mov r14, rdx
	and r14, r10
	cmp r14, qword ptr [rax]
	jb .LBB_1
	mov qword ptr [rax + 32], r14
	test r14, r14
	je .LBB_1
	mov rdi, r14
	mov rdx, rcx
	call qword ptr [rip + memmove@GOTPCREL]
	jmp .LBB_14
.LBB_4:
	mov r14, rsi
	mov rsi, r8
	mov rdx, rbx
	mov r15, rcx
	call qword ptr [rip + bumpalo::Bump::alloc_layout_slow@GOTPCREL]
	mov rsi, r14
	mov rcx, r15
	mov r14, rax
	test rax, rax
	jne .LBB_13
	xor r14d, r14d
	jmp .LBB_14