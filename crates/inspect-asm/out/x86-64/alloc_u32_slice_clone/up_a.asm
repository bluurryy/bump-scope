inspect_asm::alloc_u32_slice_clone::up_a:
	push r15
	push r14
	push rbx
	lea rbx, [4*rdx]
	mov rax, qword ptr [rdi]
	mov r14, qword ptr [rax]
	mov rcx, qword ptr [rax + 8]
	sub rcx, r14
	cmp rbx, rcx
	ja .LBB0_2
	lea rcx, [rbx + r14]
	mov qword ptr [rax], rcx
	test rdx, rdx
	je .LBB0_1
.LBB0_0:
	mov rdi, r14
	mov r15, rdx
	mov rdx, rbx
	call qword ptr [rip + memcpy@GOTPCREL]
	mov rdx, r15
.LBB0_1:
	mov rax, r14
	pop rbx
	pop r14
	pop r15
	ret
.LBB0_2:
	mov r14, rsi
	mov rsi, rdx
	mov r15, rdx
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::do_alloc_slice_in_another_chunk
	mov rsi, r14
	mov rdx, r15
	mov r14, rax
	test rdx, rdx
	jne .LBB0_0
	jmp .LBB0_1
