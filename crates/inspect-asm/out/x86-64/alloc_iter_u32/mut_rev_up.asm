inspect_asm::alloc_iter_u32::mut_rev_up:
	push rbp
	push r15
	push r14
	push r13
	push r12
	push rbx
	sub rsp, 40
	mov ebx, 4
	test rdx, rdx
	je .LBB_13
	mov r14, rdx
	mov rax, rdx
	shr rax, 61
	jne .LBB_18
	mov r15, rsi
	shl r14, 2
	mov rcx, qword ptr [rdi]
	mov rax, qword ptr [rcx]
	mov rcx, qword ptr [rcx + 8]
	add rax, 3
	and rax, -4
	mov rdx, rcx
	sub rdx, rax
	cmp rdx, r14
	jb .LBB_17
	test rax, rax
	je .LBB_17
	and rcx, -4
.LBB_5:
	mov rdx, rcx
	sub rdx, rax
	shr rdx, 2
	mov qword ptr [rsp + 8], rcx
	mov qword ptr [rsp + 16], rdi
	mov qword ptr [rsp + 24], 0
	mov qword ptr [rsp + 32], rdx
	xor r13d, r13d
	lea r12, [rsp + 8]
	xor edx, edx
	jmp .LBB_7
.LBB_6:
	mov rax, rdx
	not rax
	mov dword ptr [rcx + 4*rax], ebp
	inc rdx
	mov qword ptr [rsp + 24], rdx
	add r13, 4
	cmp r14, r13
	je .LBB_9
.LBB_7:
	mov ebp, dword ptr [r15 + r13]
	cmp qword ptr [rsp + 32], rdx
	jne .LBB_6
	mov esi, 1
	mov rdi, r12
	call bump_scope::mut_bump_vec_rev::MutBumpVecRev<T,A,_,_,_>::generic_grow_cold
	mov rcx, qword ptr [rsp + 8]
	mov rdx, qword ptr [rsp + 24]
	jmp .LBB_6
.LBB_9:
	mov rdi, qword ptr [rsp + 32]
	test rdi, rdi
	je .LBB_13
	mov r14, qword ptr [rsp + 16]
	mov rcx, qword ptr [rsp + 8]
	lea rsi, [rcx + 4*rax]
	shl rdi, 2
	mov rbx, rcx
	sub rbx, rdi
	lea r15, [rbx + 4*rdx]
	cmp r15, rsi
	jbe .LBB_14
	mov rbx, rsi
	mov r15, rcx
	jmp .LBB_15
.LBB_13:
	xor edx, edx
	jmp .LBB_16
.LBB_14:
	lea rax, [4*rdx]
	mov rdi, rbx
	mov r12, rdx
	mov rdx, rax
	call qword ptr [rip + memcpy@GOTPCREL]
	mov rdx, r12
.LBB_15:
	mov rax, qword ptr [r14]
	mov qword ptr [rax], r15
.LBB_16:
	mov rax, rbx
	add rsp, 40
	pop rbx
	pop r12
	pop r13
	pop r14
	pop r15
	pop rbp
	ret
.LBB_17:
	mov esi, 4
	mov r12, rdi
	mov rdx, r14
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::alloc_greedy_in_another_chunk
	mov rdi, r12
	mov rcx, rdx
	jmp .LBB_5
.LBB_18:
	call qword ptr [rip + bump_scope::private::capacity_overflow@GOTPCREL]