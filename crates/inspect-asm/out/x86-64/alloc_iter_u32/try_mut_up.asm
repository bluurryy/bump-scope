inspect_asm::alloc_iter_u32::try_mut_up:
	push rbp
	push r15
	push r14
	push r13
	push r12
	push rbx
	sub rsp, 40
	mov ebx, 4
	test rdx, rdx
	je .LBB_14
	mov r14, rdx
	mov rax, rdx
	shr rax, 61
	je .LBB_4
.LBB_2:
	xor ebx, ebx
	jmp .LBB_15
.LBB_4:
	mov r15, rsi
	shl r14, 2
	mov rcx, qword ptr [rdi]
	mov rax, qword ptr [rcx]
	mov rdx, qword ptr [rcx + 8]
	add rax, 3
	and rax, -4
	mov rcx, rdx
	sub rcx, rax
	cmp rcx, r14
	jb .LBB_16
	test rax, rax
	je .LBB_16
	and rdx, -4
.LBB_7:
	sub rdx, rax
	shr rdx, 2
	mov qword ptr [rsp + 8], rax
	mov qword ptr [rsp + 16], 0
	mov qword ptr [rsp + 24], rdx
	mov qword ptr [rsp + 32], rdi
	xor r13d, r13d
	lea r12, [rsp + 8]
	xor edx, edx
	jmp .LBB_9
.LBB_8:
	mov dword ptr [rax + 4*rdx], ebp
	inc rdx
	mov qword ptr [rsp + 16], rdx
	add r13, 4
	cmp r14, r13
	je .LBB_12
.LBB_9:
	mov ebp, dword ptr [r15 + r13]
	cmp qword ptr [rsp + 24], rdx
	jne .LBB_8
	mov rdi, r12
	call bump_scope::mut_bump_vec::MutBumpVec<T,A,_,_,_>::generic_grow_cold
	test al, al
	jne .LBB_2
	mov rax, qword ptr [rsp + 8]
	mov rdx, qword ptr [rsp + 16]
	jmp .LBB_8
.LBB_12:
	cmp qword ptr [rsp + 24], 0
	je .LBB_14
	mov rbx, qword ptr [rsp + 8]
	mov rax, qword ptr [rsp + 32]
	lea rcx, [rbx + 4*rdx]
	mov rax, qword ptr [rax]
	mov qword ptr [rax], rcx
	jmp .LBB_15
.LBB_14:
	xor edx, edx
.LBB_15:
	mov rax, rbx
	add rsp, 40
	pop rbx
	pop r12
	pop r13
	pop r14
	pop r15
	pop rbp
	ret
.LBB_16:
	mov esi, 4
	mov r12, rdi
	mov rdx, r14
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::alloc_greedy_in_another_chunk
	test rax, rax
	je .LBB_2
	mov rdi, r12
	jmp .LBB_7