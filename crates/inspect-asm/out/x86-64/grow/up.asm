inspect_asm::grow::up:
	push r15
	push r14
	push rbx
	mov rbx, r9
	mov rax, qword ptr [rdi]
	mov r9, qword ptr [rax]
	mov rdx, qword ptr [rax + 8]
	lea r10, [r8 - 1]
	test r10, rsi
	jne .LBB_4
	lea r10, [rsi + rcx]
	cmp r10, r9
	jne .LBB_4
	sub rdx, rsi
	cmp rdx, rbx
	jb .LBB_11
	lea rcx, [rsi + rbx]
	mov qword ptr [rax], rcx
	jmp .LBB_8
.LBB_4:
	dec r9
	mov r14, r8
	neg r14
	and r14, r9
	lea r10, [rbx + r8]
	add r10, r14
	mov r9, -1
	cmovae r9, r10
	cmp r9, rdx
	ja .LBB_9
	add r14, r8
	mov qword ptr [rax], r9
.LBB_6:
	mov rdi, r14
	mov rdx, rcx
.LBB_7:
	call qword ptr [rip + memcpy@GOTPCREL]
	mov rsi, r14
.LBB_8:
	mov rax, rsi
	mov rdx, rbx
	pop rbx
	pop r14
	pop r15
	ret
.LBB_9:
	mov r14, rsi
	mov rsi, r8
	mov rdx, rbx
	mov r15, rcx
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::alloc_in_another_chunk
	mov rcx, r15
	mov rsi, r14
	mov r14, rax
	test rax, rax
	jne .LBB_6
	jmp .LBB_10
.LBB_11:
	mov r14, rcx
	mov r15, rsi
	mov rsi, r8
	mov rdx, rbx
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::alloc_in_another_chunk
	test rax, rax
	je .LBB_10
	mov rdi, rax
	mov rsi, r15
	mov rdx, r14
	mov r14, rax
	jmp .LBB_7
.LBB_10:
	xor esi, esi
	jmp .LBB_8