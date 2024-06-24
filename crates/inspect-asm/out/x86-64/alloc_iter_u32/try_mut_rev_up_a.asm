inspect_asm::alloc_iter_u32::try_mut_rev_up_a:
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
	jmp .LBB0_9
.LBB0_1:
	mov r15, rsi
	shl r14, 2
	mov rcx, qword ptr [rdi]
	mov rax, qword ptr [rcx]
	mov rcx, qword ptr [rcx + 8]
	mov rdx, rcx
	sub rdx, rax
	cmp rdx, r14
	jb .LBB0_10
	and rcx, -4
.LBB0_2:
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
	jmp .LBB0_4
.LBB0_3:
	mov rax, rdx
	not rax
	mov dword ptr [rcx + 4*rax], ebp
	inc rdx
	mov qword ptr [rsp + 24], rdx
	add r13, 4
	cmp r14, r13
	je .LBB0_5
.LBB0_4:
	mov ebp, dword ptr [r15 + r13]
	cmp qword ptr [rsp + 32], rdx
	jne .LBB0_3
	mov rdi, r12
	call bump_scope::mut_bump_vec_rev::MutBumpVecRev<T,A,_,_,_>::generic_grow_cold
	test al, al
	jne .LBB0_0
	mov rcx, qword ptr [rsp + 8]
	mov rdx, qword ptr [rsp + 24]
	jmp .LBB0_3
.LBB0_5:
	mov rdi, qword ptr [rsp + 32]
	test rdi, rdi
	je .LBB0_6
	mov rcx, qword ptr [rsp + 8]
	mov r14, qword ptr [rsp + 16]
	lea rsi, [rcx + 4*rax]
	shl rdi, 2
	mov rbx, rcx
	sub rbx, rdi
	lea r15, [rbx + 4*rdx]
	cmp r15, rsi
	jbe .LBB0_7
	mov rbx, rsi
	mov r15, rcx
	jmp .LBB0_8
.LBB0_6:
	xor edx, edx
	jmp .LBB0_9
.LBB0_7:
	lea rax, [4*rdx]
	mov rdi, rbx
	mov r12, rdx
	mov rdx, rax
	call qword ptr [rip + memcpy@GOTPCREL]
	mov rdx, r12
.LBB0_8:
	mov rax, qword ptr [r14]
	mov qword ptr [rax], r15
.LBB0_9:
	mov rax, rbx
	add rsp, 40
	pop rbx
	pop r12
	pop r13
	pop r14
	pop r15
	pop rbp
	ret
.LBB0_10:
	mov esi, 4
	mov r12, rdi
	mov rdx, r14
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::alloc_greedy_in_another_chunk
	test rax, rax
	je .LBB0_0
	mov rdi, r12
	mov rcx, rdx
	jmp .LBB0_2
