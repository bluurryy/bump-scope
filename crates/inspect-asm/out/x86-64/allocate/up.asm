inspect_asm::allocate::up:
	mov rcx, qword ptr [rdi]
	mov r8, qword ptr [rcx]
	dec r8
	mov rax, rsi
	neg rax
	and rax, r8
	lea r9, [rdx + rsi]
	add r9, rax
	mov r8, -1
	cmovae r8, r9
	cmp r8, qword ptr [rcx + 8]
	ja .LBB0_0
	mov qword ptr [rcx], r8
	add rax, rsi
	je .LBB0_0
	ret
.LBB0_0:
	push rbx
	mov rbx, rdx
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::alloc_in_another_chunk
	mov rdx, rbx
	pop rbx
	ret
