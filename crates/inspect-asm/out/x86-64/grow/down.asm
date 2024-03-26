inspect_asm::grow::down:
	push r15
	push r14
	push r12
	push rbx
	push rax
	mov rbx, r9
	mov rax, qword ptr [rdi]
	mov rdx, qword ptr [rax]
	cmp rdx, rsi
	je .LBB_4
	xor r9d, r9d
	sub rdx, rbx
	cmovae r9, rdx
	mov r14, r8
	neg r14
	and r14, r9
	cmp r14, qword ptr [rax + 8]
	jb .LBB_10
	mov qword ptr [rax], r14
.LBB_3:
	mov rdi, r14
	mov rdx, rcx
	call qword ptr [rip + memcpy@GOTPCREL]
	jmp .LBB_9
.LBB_4:
	mov rdx, rbx
	sub rdx, rcx
	xor r9d, r9d
	cmp r8, 1
	mov r10d, 0
	sbb r10, r8
	mov r14, rsi
	sub r14, rdx
	cmovb r14, r9
	and r14, r10
	cmp r14, qword ptr [rax + 8]
	jb .LBB_12
	mov r15, rdi
	lea rax, [r14 + rbx]
	mov rdi, r14
	mov rdx, rcx
	cmp rax, rsi
	jae .LBB_7
	call qword ptr [rip + memcpy@GOTPCREL]
	jmp .LBB_8
.LBB_7:
	call qword ptr [rip + memmove@GOTPCREL]
.LBB_8:
	mov rax, qword ptr [r15]
	mov qword ptr [rax], r14
.LBB_9:
	mov rax, r14
	mov rdx, rbx
	add rsp, 8
	pop rbx
	pop r12
	pop r14
	pop r15
	ret
.LBB_10:
	mov r14, rsi
	mov rsi, r8
	mov rdx, rbx
	mov r15, rcx
	call bump_scope::bump_scope::BumpScope<_,_,A>::alloc_in_another_chunk
	mov rsi, r14
	mov rcx, r15
	mov r14, rax
	test rax, rax
	jne .LBB_3
	jmp .LBB_11
.LBB_12:
	mov r12, rsi
	mov r15, rcx
	mov rsi, r8
	mov rdx, rbx
	call bump_scope::bump_scope::BumpScope<_,_,A>::alloc_in_another_chunk
	test rax, rax
	je .LBB_11
	mov r14, rax
	mov rdi, rax
	mov rsi, r12
	mov rdx, r15
	call qword ptr [rip + memcpy@GOTPCREL]
	jmp .LBB_9
.LBB_11:
	xor r14d, r14d
	jmp .LBB_9