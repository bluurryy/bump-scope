inspect_asm::alloc_big::try_down:
	push rbx
	mov rax, qword ptr [rdi]
	mov rcx, qword ptr [rax]
	xor ebx, ebx
	sub rcx, 512
	cmovae rbx, rcx
	and rbx, -512
	cmp rbx, qword ptr [rax + 8]
	jb .LBB_2
	mov qword ptr [rax], rbx
	test rbx, rbx
	je .LBB_2
.LBB_4:
	mov edx, 512
	mov rdi, rbx
	call qword ptr [rip + memcpy@GOTPCREL]
	mov rax, rbx
	pop rbx
	ret
.LBB_2:
	mov rbx, rsi
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::do_alloc_sized_in_another_chunk
	mov rsi, rbx
	mov rbx, rax
	test rax, rax
	jne .LBB_4
	xor ebx, ebx
	mov rax, rbx
	pop rbx
	ret