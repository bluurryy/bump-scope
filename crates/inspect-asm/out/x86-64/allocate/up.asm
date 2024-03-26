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
	ja .LBB_2
	add rax, rsi
	mov qword ptr [rcx], r8
	ret
.LBB_2:
	push rbx
	mov rbx, rdx
	call bump_scope::bump_scope::BumpScope<A,_,_>::alloc_in_another_chunk
	mov rdx, rbx
	pop rbx
	ret