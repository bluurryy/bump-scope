inspect_asm::alloc_big::try_down_a:
	push rbx
	mov rax, qword ptr [rdi]
	mov rcx, qword ptr [rax]
	xor ebx, ebx
	sub rcx, 512
	cmovae rbx, rcx
	and rbx, -512
	cmp rbx, qword ptr [rax + 8]
	jb .LBB0_0
	mov qword ptr [rax], rbx
	mov edx, 512
	mov rdi, rbx
	call qword ptr [rip + memcpy@GOTPCREL]
	test rbx, rbx
	je .LBB0_1
	mov rax, rbx
	pop rbx
	ret
.LBB0_0:
	mov rbx, rsi
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::do_alloc_sized_in_another_chunk
	test rax, rax
	je .LBB0_1
	mov edx, 512
	mov rdi, rax
	mov rsi, rbx
	pop rbx
	jmp qword ptr [rip + memcpy@GOTPCREL]
.LBB0_1:
	xor ebx, ebx
	mov rax, rbx
	pop rbx
	ret
