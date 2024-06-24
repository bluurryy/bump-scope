inspect_asm::alloc_u32_slice_clone::try_up_a:
	push r15
	push r14
	push rbx
	lea r14, [4*rdx]
	mov rax, qword ptr [rdi]
	mov rbx, qword ptr [rax]
	mov rcx, qword ptr [rax + 8]
	sub rcx, rbx
	cmp r14, rcx
	ja .LBB0_2
	lea rcx, [r14 + rbx]
	mov qword ptr [rax], rcx
.LBB0_0:
	test rdx, rdx
	je .LBB0_1
	mov rdi, rbx
	mov r15, rdx
	mov rdx, r14
	call qword ptr [rip + memcpy@GOTPCREL]
	mov rdx, r15
.LBB0_1:
	mov rax, rbx
	pop rbx
	pop r14
	pop r15
	ret
.LBB0_2:
	mov rbx, rsi
	mov rsi, rdx
	mov r15, rdx
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::do_alloc_slice_in_another_chunk
	mov rsi, rbx
	mov rdx, r15
	mov rbx, rax
	test rax, rax
	jne .LBB0_0
	xor ebx, ebx
	jmp .LBB0_1
