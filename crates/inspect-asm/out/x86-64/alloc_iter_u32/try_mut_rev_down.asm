inspect_asm::alloc_iter_u32::try_mut_rev_down:
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
	mov rax, qword ptr [rdi]
	mov rdx, qword ptr [rax]
	mov rax, qword ptr [rax + 8]
	and rdx, -4
	mov rcx, rdx
	sub rcx, rax
	cmp r14, rcx
	ja .LBB0_8
	add rax, 3
	and rax, -4
	je .LBB0_8
.LBB0_2:
	mov rcx, rdx
	sub rcx, rax
	shr rcx, 2
	mov qword ptr [rsp + 8], rdx
	mov qword ptr [rsp + 16], rdi
	mov qword ptr [rsp + 24], 0
	mov qword ptr [rsp + 32], rcx
	xor r13d, r13d
	lea r12, [rsp + 8]
	xor ecx, ecx
	jmp .LBB0_4
.LBB0_3:
	mov rax, rcx
	not rax
	mov dword ptr [rdx + 4*rax], ebp
	inc rcx
	mov qword ptr [rsp + 24], rcx
	add r13, 4
	cmp r14, r13
	je .LBB0_5
.LBB0_4:
	mov ebp, dword ptr [r15 + r13]
	cmp qword ptr [rsp + 32], rcx
	jne .LBB0_3
	mov rdi, r12
	call bump_scope::mut_bump_vec_rev::MutBumpVecRev<T,A,_,_,_>::generic_grow_amortized
	test al, al
	jne .LBB0_0
	mov rdx, qword ptr [rsp + 8]
	mov rcx, qword ptr [rsp + 24]
	jmp .LBB0_3
.LBB0_5:
	cmp qword ptr [rsp + 32], 0
	je .LBB0_6
	shl rax, 2
	add rax, qword ptr [rsp + 8]
	mov rdx, qword ptr [rsp + 16]
	mov rdx, qword ptr [rdx]
	mov qword ptr [rdx], rax
	mov rbx, rax
	jmp .LBB0_7
.LBB0_6:
	xor ecx, ecx
.LBB0_7:
	mov rax, rbx
	mov rdx, rcx
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
