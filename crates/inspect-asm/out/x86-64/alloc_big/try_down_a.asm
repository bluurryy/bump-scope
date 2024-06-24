inspect_asm::alloc_big::try_down_a:
	push rbx
	mov rax, qword ptr [rdi]
	mov rcx, qword ptr [rax]
	xor ebx, ebx
	sub rcx, 512
	cmovae rbx, rcx
	and rbx, -512
	cmp rbx, qword ptr [rax + 8]
	jb .LBB0_1
	mov qword ptr [rax], rbx
	test rbx, rbx
	je .LBB0_1
.LBB0_0:
	mov edx, 512
	mov rdi, rbx
	call qword ptr [rip + memcpy@GOTPCREL]
	mov rax, rbx
	pop rbx
	ret
.LBB0_1:
	mov rbx, rsi
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::do_alloc_sized_in_another_chunk
	mov rsi, rbx
	mov rbx, rax
	test rax, rax
	jne .LBB0_0
	xor ebx, ebx
	mov rax, rbx
	pop rbx
	ret
