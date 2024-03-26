inspect_asm::alloc_u32_slice_clone::try_down:
	push r15
	push r14
	push rbx
	lea r14, [4*rdx]
	mov rax, qword ptr [rdi]
	mov rbx, qword ptr [rax]
	mov rcx, rbx
	sub rcx, qword ptr [rax + 8]
	cmp r14, rcx
	ja .LBB_4
	sub rbx, r14
	and rbx, -4
	mov qword ptr [rax], rbx
.LBB_2:
	test rdx, rdx
	je .LBB_6
	mov rdi, rbx
	mov r15, rdx
	mov rdx, r14
	call qword ptr [rip + memcpy@GOTPCREL]
	mov rdx, r15
.LBB_6:
	mov rax, rbx
	pop rbx
	pop r14
	pop r15
	ret
.LBB_4:
	mov rbx, rsi
	mov rsi, rdx
	mov r15, rdx
	call bump_scope::bump_scope::BumpScope<_,_,A>::do_alloc_slice_in_another_chunk
	mov rsi, rbx
	mov rdx, r15
	mov rbx, rax
	test rax, rax
	jne .LBB_2
	xor ebx, ebx
	jmp .LBB_6