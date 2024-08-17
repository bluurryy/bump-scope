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
	je .LBB0_1
	xor r9d, r9d
	sub rdx, rbx
	cmovae r9, rdx
	mov r14, r8
	neg r14
	and r14, r9
	cmp r14, qword ptr [rax + 8]
	jb .LBB0_5
	mov qword ptr [rax], r14
	test r14, r14
	je .LBB0_5
.LBB0_0:
	mov rdi, r14
	mov rdx, rcx
	call qword ptr [rip + memcpy@GOTPCREL]
	jmp .LBB0_4
.LBB0_1:
	mov rdx, rbx
	sub rdx, rcx
	xor r9d, r9d
	mov r10, rsi
	sub r10, rdx
	cmovae r9, r10
	mov r14, r8
	neg r14
	and r14, r9
	cmp r14, qword ptr [rax + 8]
	jb .LBB0_6
	mov r15, rdi
	lea rax, [r14 + rbx]
	mov rdi, r14
	mov rdx, rcx
	cmp rax, rsi
	jae .LBB0_2
	call qword ptr [rip + memcpy@GOTPCREL]
	jmp .LBB0_3
.LBB0_2:
	call qword ptr [rip + memmove@GOTPCREL]
.LBB0_3:
	mov rax, qword ptr [r15]
	mov qword ptr [rax], r14
.LBB0_4:
	mov rax, r14
	mov rdx, rbx
	add rsp, 8
	pop rbx
	pop r12
	pop r14
	pop r15
	ret
.LBB0_5:
	mov r14, rsi
	mov rsi, r8
	mov rdx, rbx
	mov r15, rcx
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::alloc_in_another_chunk
	mov rsi, r14
	mov rcx, r15
	mov r14, rax
	test rax, rax
	jne .LBB0_0
	jmp .LBB0_7
.LBB0_6:
	mov r12, rsi
	mov r15, rcx
	mov rsi, r8
	mov rdx, rbx
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::alloc_in_another_chunk
	test rax, rax
	je .LBB0_7
	mov r14, rax
	mov rdi, rax
	mov rsi, r12
	mov rdx, r15
	call qword ptr [rip + memcpy@GOTPCREL]
	jmp .LBB0_4
.LBB0_7:
	xor r14d, r14d
	jmp .LBB0_4
