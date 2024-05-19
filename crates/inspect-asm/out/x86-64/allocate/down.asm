inspect_asm::allocate::down:
	mov rcx, qword ptr [rdi]
	mov rax, qword ptr [rcx]
	xor r8d, r8d
	sub rax, rdx
	cmovae r8, rax
	mov rax, rsi
	neg rax
	and rax, r8
	cmp rax, qword ptr [rcx + 8]
	jb .LBB_2
	mov qword ptr [rcx], rax
	test rax, rax
	je .LBB_2
	ret
.LBB_2:
	push rbx
	mov rbx, rdx
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::alloc_in_another_chunk
	mov rdx, rbx
	pop rbx
	ret