inspect_asm::alloc_u32_slice::try_down_a:
	push r15
	push r14
	push rbx
	mov rbx, rdx
	lea rdx, [4*rdx]
	mov rax, qword ptr [rdi]
	mov r14, qword ptr [rax]
	mov rcx, r14
	sub rcx, qword ptr [rax + 8]
	cmp rdx, rcx
	ja .LBB_3
	sub r14, rdx
	mov qword ptr [rax], r14
.LBB_2:
	mov rdi, r14
	call qword ptr [rip + memcpy@GOTPCREL]
.LBB_5:
	mov rax, r14
	mov rdx, rbx
	pop rbx
	pop r14
	pop r15
	ret
.LBB_3:
	mov r14, rsi
	mov rsi, rbx
	mov r15, rdx
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::do_alloc_slice_in_another_chunk
	mov rdx, r15
	mov rsi, r14
	mov r14, rax
	test rax, rax
	jne .LBB_2
	xor r14d, r14d
	jmp .LBB_5