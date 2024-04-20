inspect_asm::alloc_iter_u32::try_mut_rev_down:
	push rbp
	push r15
	push r14
	push r12
	push rbx
	sub rsp, 32
	test rdx, rdx
	je .LBB_12
	mov rbx, rdx
	mov rax, rdx
	shr rax, 61
	je .LBB_2
.LBB_13:
	xor eax, eax
	jmp .LBB_15
.LBB_2:
	mov r14, rsi
	shl rbx, 2
	mov rax, qword ptr [rdi]
	mov rdx, qword ptr [rax]
	mov rax, qword ptr [rax + 8]
	and rdx, -4
	mov rcx, rdx
	sub rcx, rax
	cmp rcx, rbx
	jb .LBB_4
	add rax, 3
	and rax, -4
.LBB_6:
	mov rcx, rdx
	sub rcx, rax
	shr rcx, 2
	mov qword ptr [rsp], rdx
	mov qword ptr [rsp + 8], rdi
	mov qword ptr [rsp + 16], 0
	mov qword ptr [rsp + 24], rcx
	xor r12d, r12d
	mov r15, rsp
	xor ecx, ecx
	jmp .LBB_7
.LBB_10:
	mov rax, rcx
	not rax
	mov dword ptr [rdx + 4*rax], ebp
	inc rcx
	mov qword ptr [rsp + 16], rcx
	add r12, 4
	cmp rbx, r12
	je .LBB_11
.LBB_7:
	mov ebp, dword ptr [r14 + r12]
	cmp qword ptr [rsp + 24], rcx
	jne .LBB_10
	mov rdi, r15
	call bump_scope::mut_bump_vec_rev::MutBumpVecRev<T,A,_,_>::generic_grow_cold
	test al, al
	jne .LBB_13
	mov rdx, qword ptr [rsp]
	mov rcx, qword ptr [rsp + 16]
	jmp .LBB_10
.LBB_11:
	cmp qword ptr [rsp + 24], 0
	je .LBB_12
	mov rdx, qword ptr [rsp + 8]
	shl rax, 2
	add rax, qword ptr [rsp]
	mov rdx, qword ptr [rdx]
	mov qword ptr [rdx], rax
	jmp .LBB_15
.LBB_12:
	mov eax, 4
	xor ecx, ecx
.LBB_15:
	mov rdx, rcx
	add rsp, 32
	pop rbx
	pop r12
	pop r14
	pop r15
	pop rbp
	ret
.LBB_4:
	mov esi, 4
	mov r15, rdi
	mov rdx, rbx
	call bump_scope::bump_scope::BumpScope<A,_,_>::alloc_greedy_in_another_chunk
	test rax, rax
	je .LBB_13
	mov rdi, r15
	jmp .LBB_6