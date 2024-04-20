inspect_asm::grow::bumpalo:
	push r15
	push r14
	push rbx
	mov rbx, r9
	mov rax, qword ptr [rdi + 16]
	mov r9, qword ptr [rax + 32]
	cmp rdx, r8
	jb .LBB_2
	cmp r9, rsi
	jne .LBB_2
	mov r10, rbx
	sub r10, rcx
	lea r9, [rdx - 1]
	mov r11, rdx
	xor r11, r9
	movabs r15, -9223372036854775808
	sub r15, rdx
	xor r14d, r14d
	cmp r15, r10
	cmovb rdx, r14
	cmp r11, r9
	jbe .LBB_13
	test rdx, rdx
	je .LBB_13
	mov r9, rsi
	cmp r10, rsi
	ja .LBB_2
	mov r9, rsi
	sub r9, r10
	neg rdx
	mov r14, rdx
	and r14, r9
	mov r9, rsi
	cmp r14, qword ptr [rax]
	jae .LBB_12
.LBB_2:
	cmp r9, rbx
	jb .LBB_6
	lea r14, [r8 - 1]
	sub r9, rbx
	not r14
	and r14, r9
	cmp r14, qword ptr [rax]
	jb .LBB_6
	mov qword ptr [rax + 32], r14
.LBB_5:
	mov rdi, r14
	mov rdx, rcx
	call qword ptr [rip + memcpy@GOTPCREL]
.LBB_13:
	mov rax, r14
	mov rdx, rbx
	pop rbx
	pop r14
	pop r15
	ret
.LBB_6:
	mov r14, rsi
	mov rsi, r8
	mov rdx, rbx
	mov r15, rcx
	call qword ptr [rip + bumpalo::Bump::alloc_layout_slow@GOTPCREL]
	mov rsi, r14
	mov rcx, r15
	mov r14, rax
	test rax, rax
	jne .LBB_5
	xor r14d, r14d
	jmp .LBB_13
.LBB_12:
	mov qword ptr [rax + 32], r14
	mov rdi, r14
	mov rdx, rcx
	call qword ptr [rip + memmove@GOTPCREL]
	jmp .LBB_13