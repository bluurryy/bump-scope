inspect_asm::alloc_iter_u32::rev_up:
	push rbp
	push r15
	push r14
	push r13
	push r12
	push rbx
	sub rsp, 40
	mov ebx, 4
	test rdx, rdx
	je .LBB_11
	mov r14, rdx
	mov r15, rsi
	mov r12, rdi
	shl r14, 2
	mov rcx, qword ptr [rdi]
	mov rax, qword ptr [rcx]
	mov rdx, qword ptr [rcx + 8]
	add rax, 3
	and rax, -4
	mov rcx, rdx
	sub rcx, rax
	cmp rcx, r14
	jb .LBB_15
	and rdx, -4
.LBB_3:
	mov rcx, rdx
	sub rcx, rax
	shr rcx, 2
	mov qword ptr [rsp + 8], rdx
	mov qword ptr [rsp + 16], r12
	mov qword ptr [rsp + 24], 0
	mov qword ptr [rsp + 32], rcx
	xor ebp, ebp
	xor r12d, r12d
	jmp .LBB_5
.LBB_4:
	mov rax, r12
	not rax
	mov dword ptr [rdx + 4*rax], r13d
	inc r12
	mov qword ptr [rsp + 24], r12
	add rbp, 4
	cmp r14, rbp
	je .LBB_7
.LBB_5:
	mov r13d, dword ptr [r15 + rbp]
	cmp qword ptr [rsp + 32], r12
	jne .LBB_4
	mov esi, 1
	lea rdi, [rsp + 8]
	call bump_scope::mut_bump_vec_rev::MutBumpVecRev<T,_,_,A>::generic_grow_cold
	mov rdx, qword ptr [rsp + 8]
	mov r12, qword ptr [rsp + 24]
	jmp .LBB_4
.LBB_7:
	mov rdx, qword ptr [rsp + 32]
	test rdx, rdx
	je .LBB_11
	mov r14, qword ptr [rsp + 16]
	mov rcx, qword ptr [rsp + 8]
	lea rsi, [rcx + 4*rax]
	shl rdx, 2
	mov rbx, rcx
	sub rbx, rdx
	lea r15, [rbx + 4*r12]
	cmp r15, rsi
	jbe .LBB_12
	mov rbx, rsi
	mov r15, rcx
	jmp .LBB_13
.LBB_11:
	xor r12d, r12d
	jmp .LBB_14
.LBB_12:
	lea rdx, [4*r12]
	mov rdi, rbx
	call qword ptr [rip + memcpy@GOTPCREL]
.LBB_13:
	mov rax, qword ptr [r14]
	mov qword ptr [rax], r15
.LBB_14:
	mov rax, rbx
	mov rdx, r12
	add rsp, 40
	pop rbx
	pop r12
	pop r13
	pop r14
	pop r15
	pop rbp
	ret
.LBB_15:
	mov esi, 4
	mov rdi, r12
	mov rdx, r14
	call bump_scope::bump_scope::BumpScope<_,_,A>::alloc_greedy_in_another_chunk
	jmp .LBB_3