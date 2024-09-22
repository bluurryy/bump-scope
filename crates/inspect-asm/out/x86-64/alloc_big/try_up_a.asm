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
	ja .LBB0_0
	mov qword ptr [rax], rcx
	lea rdi, [rbx + 512]
	mov edx, 512
	call qword ptr [rip + memcpy@GOTPCREL]
	add rbx, 512
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
