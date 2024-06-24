inspect_asm::alloc_iter_u32::try_mut_rev_up_a:
	push rbp
	push r15
	push r14
	push r12
	push rbx
	sub rsp, 32
	test rdx, rdx
	je .LBB0_6
	mov rbx, rdx
	mov rax, rdx
	shr rax, 61
	je .LBB0_1
.LBB0_0:
	xor eax, eax
	jmp .LBB0_9
.LBB0_1:
	mov r14, rsi
	shl rbx, 2
	mov rcx, qword ptr [rdi]
	mov rax, qword ptr [rcx]
	mov rcx, qword ptr [rcx + 8]
	mov rdx, rcx
	sub rdx, rax
	cmp rdx, rbx
	jb .LBB0_10
	and rcx, -4
.LBB0_2:
	mov rdx, rcx
	sub rdx, rax
	shr rdx, 2
	mov qword ptr [rsp], rcx
	mov qword ptr [rsp + 8], rdi
	mov qword ptr [rsp + 16], 0
	mov qword ptr [rsp + 24], rdx
	xor r12d, r12d
	mov r15, rsp
	xor edx, edx
	jmp .LBB0_4
.LBB0_3:
	mov rax, rdx
	not rax
	mov dword ptr [rcx + 4*rax], ebp
	inc rdx
	mov qword ptr [rsp + 16], rdx
	add r12, 4
	cmp rbx, r12
	je .LBB0_5
.LBB0_4:
	mov ebp, dword ptr [r14 + r12]
	cmp qword ptr [rsp + 24], rdx
	jne .LBB0_3
	mov rdi, r15
	call bump_scope::mut_bump_vec_rev::MutBumpVecRev<T,A,_,_,_>::generic_grow_cold
	test al, al
	jne .LBB0_0
	mov rcx, qword ptr [rsp]
	mov rdx, qword ptr [rsp + 16]
	jmp .LBB0_3
.LBB0_5:
	mov r8, qword ptr [rsp + 24]
	test r8, r8
	je .LBB0_6
	mov rbx, qword ptr [rsp + 8]
	mov rcx, qword ptr [rsp]
	lea rsi, [rcx + 4*rax]
	shl r8, 2
	mov rdi, rcx
	sub rdi, r8
	lea r14, [rdi + 4*rdx]
	cmp r14, rsi
	jbe .LBB0_7
	mov rax, rsi
	mov r14, rcx
	jmp .LBB0_8
.LBB0_6:
	mov eax, 4
	xor edx, edx
	jmp .LBB0_9
.LBB0_7:
	lea rax, [4*rdx]
	mov r15, rdx
	mov rdx, rax
	mov r12, rdi
	call qword ptr [rip + memcpy@GOTPCREL]
	mov rax, r12
	mov rdx, r15
.LBB0_8:
	mov rcx, qword ptr [rbx]
	mov qword ptr [rcx], r14
.LBB0_9:
	add rsp, 32
	pop rbx
	pop r12
	pop r14
	pop r15
	pop rbp
	ret
.LBB0_10:
	mov esi, 4
	mov r15, rdi
	mov rdx, rbx
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::alloc_greedy_in_another_chunk
	test rax, rax
	je .LBB0_0
	mov rdi, r15
	mov rcx, rdx
	jmp .LBB0_2
