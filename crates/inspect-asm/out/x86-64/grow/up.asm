inspect_asm::grow::up:
	push r15
	push r14
	push rbx
	mov rbx, r9
	lea r10, [rsi + rcx]
	mov rax, qword ptr [rdi]
	mov r9, qword ptr [rax]
	mov rdx, qword ptr [rax + 8]
	xor r10, r9
	lea r11, [r8 - 1]
	and r11, rsi
	or r11, r10
	jne .LBB0_0
	sub rdx, rsi
	cmp rdx, rbx
	jb .LBB0_5
	lea rcx, [rsi + rbx]
	mov qword ptr [rax], rcx
	jmp .LBB0_3
.LBB0_0:
	dec r9
	mov r14, r8
	neg r14
	and r14, r9
	lea r10, [rbx + r8]
	add r10, r14
	mov r9, -1
	cmovae r9, r10
	cmp r9, rdx
	ja .LBB0_4
	mov qword ptr [rax], r9
	add r14, r8
	je .LBB0_4
.LBB0_1:
	mov rdi, r14
	mov rdx, rcx
.LBB0_2:
	call qword ptr [rip + memcpy@GOTPCREL]
	mov rsi, r14
.LBB0_3:
	mov rax, rsi
	mov rdx, rbx
	pop rbx
	pop r14
	pop r15
	ret
.LBB0_4:
	mov r14, rsi
	mov rsi, r8
	mov rdx, rbx
	mov r15, rcx
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::alloc_in_another_chunk
	mov rcx, r15
	mov rsi, r14
	mov r14, rax
	test rax, rax
	jne .LBB0_1
	jmp .LBB0_6
.LBB0_5:
	mov r14, rcx
	mov r15, rsi
	mov rsi, r8
	mov rdx, rbx
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::alloc_in_another_chunk
	test rax, rax
	je .LBB0_6
	mov rdi, rax
	mov rsi, r15
	mov rdx, r14
	mov r14, rax
	jmp .LBB0_2
.LBB0_6:
	xor esi, esi
	jmp .LBB0_3
