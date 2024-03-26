inspect_asm::alloc_layout::try_up:
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
	ja bump_scope::bump_scope::BumpScope<A,_,_>::alloc_in_another_chunk
	add rax, rsi
	mov qword ptr [rcx], r8
	ret