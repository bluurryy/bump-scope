inspect_asm::alloc_iter_u32::try_mut_up_a:
	push rbp
	push r15
	push r14
	push r13
	push r12
	push rbx
	sub rsp, 40
	mov ebx, 4
	test rdx, rdx
	je .LBB0_6
	mov r14, rdx
	mov rax, rdx
	shr rax, 61
	je .LBB0_1
.LBB0_0:
	xor ebx, ebx
	jmp .LBB0_7
.LBB0_1:
	mov r15, rsi
	shl r14, 2
	mov rcx, qword ptr [rdi]
	mov rax, qword ptr [rcx]
	mov rdx, qword ptr [rcx + 8]
	mov rcx, rdx
	sub rcx, rax
	cmp r14, rcx
	ja .LBB0_8
	and rdx, -4
.LBB0_2:
	sub rdx, rax
	shr rdx, 2
	mov qword ptr [rsp + 8], rax
	mov qword ptr [rsp + 16], 0
	mov qword ptr [rsp + 24], rdx
	mov qword ptr [rsp + 32], rdi
	xor r13d, r13d
	lea r12, [rsp + 8]
	xor edx, edx
	jmp .LBB0_4
.LBB0_3:
	mov dword ptr [rax + 4*rdx], ebp
	inc rdx
	mov qword ptr [rsp + 16], rdx
	add r13, 4
	cmp r14, r13
	je .LBB0_5
.LBB0_4:
	mov ebp, dword ptr [r15 + r13]
	cmp qword ptr [rsp + 24], rdx
	jne .LBB0_3
	mov rdi, r12
	call bump_scope::mut_bump_vec::MutBumpVec<T,A,_,_,_>::generic_grow_cold
	test al, al
	jne .LBB0_0
	mov rax, qword ptr [rsp + 8]
	mov rdx, qword ptr [rsp + 16]
	jmp .LBB0_3
.LBB0_5:
	cmp qword ptr [rsp + 24], 0
	je .LBB0_6
	mov rbx, qword ptr [rsp + 8]
	mov rax, qword ptr [rsp + 32]
	lea rcx, [rbx + 4*rdx]
	mov rax, qword ptr [rax]
	mov qword ptr [rax], rcx
	jmp .LBB0_7
.LBB0_6:
	xor edx, edx
.LBB0_7:
	mov rax, rbx
	add rsp, 40
	pop rbx
	pop r12
	pop r13
	pop r14
	pop r15
	pop rbp
	ret
.LBB0_8:
	mov esi, 4
	mov r12, rdi
	mov rdx, r14
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::alloc_greedy_in_another_chunk
	test rax, rax
	je .LBB0_0
	mov rdi, r12
	jmp .LBB0_2
