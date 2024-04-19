inspect_asm::alloc_big::try_up_a:
	push rbx
	mov rax, qword ptr [rdi]
	mov rbx, qword ptr [rax]
	dec rbx
	and rbx, -512
	mov rdx, rbx
	add rdx, 1024
	mov rcx, -1
	cmovae rcx, rdx
	cmp rcx, qword ptr [rax + 8]
	ja .LBB_3
	add rbx, 512
	mov qword ptr [rax], rcx
.LBB_2:
	mov edx, 512
	mov rdi, rbx
	call qword ptr [rip + memcpy@GOTPCREL]
	mov rax, rbx
	pop rbx
	ret
.LBB_3:
	mov rbx, rsi
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::do_alloc_sized_in_another_chunk
	mov rsi, rbx
	mov rbx, rax
	test rax, rax
	jne .LBB_2
	xor ebx, ebx
	mov rax, rbx
	pop rbx
	ret